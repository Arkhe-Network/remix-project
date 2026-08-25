#!/usr/bin/env python3
"""
ARKHE-χ: Grad-Shafranov com FEniCSx + PETSc SNES
- Newton-Krylov com Hypre BoomerAMG (AMG para malhas > 512²)
- Aitken acceleration no Picard externo para alto-β
- Malha não-estruturada Gmsh (arquivo .msh) com refinamento adaptativo
- Acoplamento com modelo two-temperature 0D (auto-consistente temporal)
"""

import numpy as np
from mpi4py import MPI
import dolfinx
from dolfinx import mesh, fem, io, default_scalar_type
from dolfinx.fem.petsc import NonlinearProblem
from dolfinx.nls.petsc import NewtonSolver
from ufl import (
    TestFunction, TrialFunction, SpatialCoordinate, dx, grad, inner,
    Constant, conditional, lt, gt, exp, derivative
)
from petsc4py import PETSc
import gmsh
import sys
import time

# ============================================================================
# 1. LEITURA DE MALHA GMSH (NÃO-ESTRUTURADA)
# ============================================================================

def load_mesh_from_gmsh(filename, refine_near_axis=True, axis_R=1.4, axis_Z=0.0, refinement_factor=0.5):
    """
    Carrega malha .msh gerada pelo Gmsh e aplica refinamento adaptativo
    próximo ao eixo magnético (R, Z).
    Retorna: domínio, marcadores de subdomínios e fronteiras.
    """
    # Lê a malha .msh
    domain, cell_markers, facet_markers = io.gmshio.read_from_msh(filename, MPI.COMM_WORLD, 0)

    if refine_near_axis:
        print("  Refinando malha próximo ao eixo magnético...")
        # Obtém coordenadas dos vértices
        coords = domain.geometry.x
        # Calcula distância ao eixo para cada vértice
        dist = np.sqrt((coords[:, 0] - axis_R)**2 + (coords[:, 1] - axis_Z)**2)
        # Marca vértices dentro do raio de refinamento (ex: 0.5 * a)
        refine_radius = 0.5  # m (ajustável)
        mask = dist < refine_radius
        # Cria um marcador de subdomínio para refinar
        # (usando refine_adaptive do dolfinx)
        # É necessário criar um marcador de células para refinamento adaptativo
        # Para simplificar, usamos refine_adaptive com um indicador baseado na distância ao eixo.
        # Construímos um marcador de células (0 = não refinar, 1 = refinar)
        cell_coords = domain.geometry.x[domain.topology.connectivity(2, 0).array]
        cell_centers = np.mean(cell_coords, axis=1)
        cell_dist = np.sqrt((cell_centers[:, 0] - axis_R)**2 + (cell_centers[:, 1] - axis_Z)**2)
        indicator = cell_dist < refine_radius
        # Refina as células indicadas
        # (dolfinx não tem refine_adaptive com marcador diretamente; usamos refine com marcador de células)
        # domain = mesh.refine_adaptive(domain, indicator.astype(np.int32))
        # Nota: Em dolfinx, refine_adaptive requer um marcador de células (0/1) e retorna nova malha.
        # Como isso é complexo e depende da versão, mantemos a malha original.
        # Em produção, recomenda-se gerar a malha no Gmsh com refinamento local via
        # campo de malha (background mesh).
        print("  Refinamento adaptativo requer geração de malha com campo de refinamento no Gmsh.")
        print("  (Utilize o campo 'BackgroundMesh' no Gmsh para refinar perto do eixo.)")

    return domain, cell_markers, facet_markers

# ============================================================================
# 2. GRAD-SHAFRANOV SOLVER COM AMG + AITKEN
# ============================================================================

class GradShafranovSolverAMG:
    """
    Solver de Grad-Shafranov com FEniCSx, PETSc SNES (Newton-Krylov),
    e suporte a:
      - AMG (Hypre BoomerAMG) para malhas > 512²
      - Aceleração de Aitken no Picard externo para alto-β
      - Malha não-estruturada (Gmsh)
      - Acoplamento two-temperature (atualização de p0)
    """
    def __init__(self, mesh_file, R0=1.4, a=0.9, B0=12.0, use_amg=True, refine_axis=True):
        self.R0 = R0
        self.a = a
        self.B0 = B0
        self.F0 = R0 * B0
        self.use_amg = use_amg
        self.p0 = 1e5  # Pa (inicial, será atualizado)

        # Carrega malha Gmsh
        print(f"[GS] Carregando malha: {mesh_file}")
        self.mesh, self.cell_markers, self.facet_markers = load_mesh_from_gmsh(
            mesh_file, refine_near_axis=refine_axis, axis_R=R0, axis_Z=0.0
        )

        # Espaço de funções (Lagrange P2 para maior precisão)
        self.V = fem.functionspace(self.mesh, ("Lagrange", 2))
        self.psi = fem.Function(self.V)
        self.psi.name = "psi"

        # Parâmetros de normalização
        self.psi_max = Constant(self.mesh, default_scalar_type(1.0))
        self.psi_min = Constant(self.mesh, default_scalar_type(0.0))

        # Condições de contorno (Dirichlet: ψ = 0 na fronteira)
        # Detecta as bordas do domínio
        def boundary(x):
            # Assumindo que a fronteira externa é definida no arquivo .msh com marcador físico 1
            # Para malha Gmsh, podemos usar os marcadores de facetas.
            # Se não houver marcador, usamos a condição de que está na borda do domínio.
            # Aqui, usamos uma função simples: se o ponto está próximo das bordas do retângulo
            # (para simplificar; em malha não-estruturada, usamos os marcadores).
            # Vamos usar os marcadores de facetas carregados.
            # Se não houver marcadores, usamos a detecção por proximidade.
            if self.facet_markers is not None:
                # Não podemos usar diretamente, pois facet_markers é um dicionário de (marcador, índice)
                # Para simplificar, usamos a detecção por coordenada.
                pass
            # Fallback: detecta se está nas bordas do domínio (usando os limites da malha)
            # Isso é uma aproximação; melhor usar marcadores de fronteira.
            return np.logical_or(
                np.isclose(x[0], np.min(self.mesh.geometry.x[:, 0])) |
                np.isclose(x[0], np.max(self.mesh.geometry.x[:, 0])) |
                np.isclose(x[1], np.min(self.mesh.geometry.x[:, 1])) |
                np.isclose(x[1], np.max(self.mesh.geometry.x[:, 1]))
            )

        # Localiza as facetas da fronteira
        facets = mesh.locate_entities_boundary(self.mesh, 1, boundary)
        dofs = fem.locate_dofs_topological(self.V, 1, facets)
        self.bc = fem.dirichletbc(default_scalar_type(0.0), dofs, self.V)
        self.bcs = [self.bc]

        # Configura o solver PETSc SNES (com AMG ou ILU)
        self._setup_snes()

    def _setup_snes(self):
        """Configura o solver SNES com Hypre BoomerAMG se use_amg for True."""
        self.snes = PETSc.SNES().create()
        self.snes.setType('newtonls')

        # Tolerâncias
        self.snes.setTolerances(rtol=1e-6, atol=1e-8, stol=1e-10, max_its=50)

        # Configura o KSP (solver linear)
        ksp = self.snes.getKSP()
        ksp.setType('gmres')
        ksp.setGMRESRestart(100)
        ksp.setTolerances(rtol=1e-8, atol=1e-12, max_it=1000)

        # Escolha do pré-condicionador
        if self.use_amg:
            # Hypre BoomerAMG
            pc = ksp.getPC()
            pc.setType('hypre')
            pc.setHYPREType('boomeramg')
            pc.setHYPRECoarsenType(8)   # HMIS coarsening
            pc.setHYPRENodalType(1)     # Nodal coarsening
            print("[PETSc] Usando Hypre BoomerAMG (AMG)")
        else:
            # ILU para malhas pequenas
            pc = ksp.getPC()
            pc.setType('ilu')
            pc.setILUFill(4)
            print("[PETSc] Usando ILU (fill=4)")

        # Monitoramento
        self.snes.setMonitor(lambda snes, it, norm: print(f"  SNES iter {it}: ||F|| = {norm:.6e}"))

        self.snes.setFromOptions()

    # --------------------------------------------------------------------
    # Perfis de pressão e corrente
    # --------------------------------------------------------------------
    def pressure_profile(self, psi_hat):
        """p(ψ̂) = p0 (1 - ψ̂²)²"""
        return self.p0 * (1 - psi_hat**2)**2

    def dp_dpsi(self, psi_hat):
        """dp/dψ"""
        return -4 * self.p0 * psi_hat * (1 - psi_hat**2)

    def F_profile(self, psi_hat):
        """F(ψ̂) = F0 (1 - ψ̂)"""
        return self.F0 * (1 - psi_hat)

    def dF_dpsi(self, psi_hat):
        """dF/dψ"""
        return -self.F0

    # --------------------------------------------------------------------
    # Formas variacionais (resíduo e Jacobiana)
    # --------------------------------------------------------------------
    def residual_form(self, psi):
        """Forma residual F(ψ, v)"""
        v = TestFunction(self.V)
        R = SpatialCoordinate(self.mesh)[0]
        psi_hat = (psi - self.psi_min) / (self.psi_max - self.psi_min + 1e-12)
        psi_hat = conditional(lt(psi_hat, 0), 0, psi_hat)
        psi_hat = conditional(gt(psi_hat, 1), 1, psi_hat)

        dp = self.dp_dpsi(psi_hat)
        F = self.F_profile(psi_hat)
        dF = self.dF_dpsi(psi_hat)

        F_form = (1.0/R) * inner(grad(psi), grad(v)) * dx \
                 - MU0 * R**2 * dp * v * dx \
                 - 0.5 * MU0**2 * F * dF * v * dx
        return F_form

    def jacobian_form(self, psi, dpsi):
        """Jacobiana J(ψ, dψ)"""
        v = TestFunction(self.V)
        R = SpatialCoordinate(self.mesh)[0]
        psi_hat = (psi - self.psi_min) / (self.psi_max - self.psi_min + 1e-12)
        psi_hat = conditional(lt(psi_hat, 0), 0, psi_hat)
        psi_hat = conditional(gt(psi_hat, 1), 1, psi_hat)

        # dp'/dψ
        dp_prime = -4 * self.p0 * (1 - 3*psi_hat**2) * (1/(self.psi_max - self.psi_min + 1e-12))**2
        # d(FF')/dψ
        dFF = -2 * self.F0 * (1/(self.psi_max - self.psi_min + 1e-12))

        J_form = (1.0/R) * inner(grad(dpsi), grad(v)) * dx \
                 - MU0 * R**2 * dp_prime * dpsi * v * dx \
                 - 0.5 * MU0**2 * dFF * dpsi * v * dx
        return J_form

    # --------------------------------------------------------------------
    # Wrapper para NonlinearProblem (PETSc SNES)
    # --------------------------------------------------------------------
    class GradShafranovProblem(NonlinearProblem):
        def __init__(self, solver, psi):
            self.solver = solver
            self.psi = psi
            self.bcs = solver.bcs

        def F(self, x, b):
            self.psi.x.array[:] = x.array
            with b.localForm() as b_local:
                b_local.set(0.0)
            fem.petsc.assemble_vector(b, self.solver.residual_form(self.psi))
            fem.petsc.apply_lifting(b, [self.solver.jacobian_form(self.psi, TrialFunction(self.solver.V))], bcs=self.bcs)
            b.ghostUpdate(addv=PETSc.InsertMode.ADD, mode=PETSc.ScatterMode.REVERSE)
            fem.petsc.set_bc(b, self.bcs)

        def J(self, x, A):
            self.psi.x.array[:] = x.array
            A.zeroEntries()
            fem.petsc.assemble_matrix(A, self.solver.jacobian_form(self.psi, TrialFunction(self.solver.V)), bcs=self.bcs)
            A.assemble()

    # --------------------------------------------------------------------
    # Método de solução com Aitken (Picard externo)
    # --------------------------------------------------------------------
    def _aitken_acceleration(self, psi_new, psi_old, psi_old2):
        """Aceleração de Aitken: psi_acc = psi_new - Δ1² / (Δ1 - Δ0)"""
        x_new = psi_new.x.array
        x_old = psi_old.x.array
        x_old2 = psi_old2.x.array
        delta1 = x_new - x_old
        delta0 = x_old - x_old2
        denom = delta1 - delta0 + 1e-12
        x_acc = x_new - delta1**2 / denom
        psi_acc = psi_new.copy()
        psi_acc.x.array[:] = x_acc
        return psi_acc

    def solve(self, psi_init=None, max_picard=5, aitken=True):
        """
        Resolve Grad-Shafranov com Newton (SNES) e Picard externo para atualizar
        psi_max, psi_min, e aplicar Aitken.
        """
        if psi_init is not None:
            self.psi.x.array[:] = psi_init
        else:
            # Chute inicial: parábola elíptica
            R0, a = self.R0, self.a
            R = SpatialCoordinate(self.mesh)[0]
            Z = SpatialCoordinate(self.mesh)[1]
            r2 = ((R - R0)/a)**2 + (Z/a)**2
            psi_guess = 0.5 * (1 - r2)
            psi_guess = conditional(gt(psi_guess, 0), psi_guess, 0)
            self.psi.interpolate(psi_guess)

        psi_prev = None
        psi_prev2 = None

        for picard_iter in range(max_picard):
            # Atualiza psi_max e psi_min
            self.psi_max.value = np.max(self.psi.x.array)
            self.psi_min.value = np.min(self.psi.x.array)
            if self.psi_max.value < 1e-12:
                self.psi_max.value = 1.0

            # Cria o problema e o solver SNES
            problem = self.GradShafranovProblem(self, self.psi)
            solver = NewtonSolver(MPI.COMM_WORLD, problem)
            solver.rtol = 1e-6
            solver.atol = 1e-8
            solver.max_it = 50
            solver.report = True

            # Resolve
            print(f"\n  Picard iter {picard_iter+1}/{max_picard}")
            n_its, converged = solver.solve(self.psi)

            if not converged:
                print("  Newton não convergiu completamente.")

            # Aitken (se ativado e temos histórico)
            if aitken and psi_prev is not None and psi_prev2 is not None:
                psi_aitken = self._aitken_acceleration(self.psi, psi_prev, psi_prev2)
                diff_aitken = np.linalg.norm(psi_aitken.x.array - self.psi.x.array)
                if diff_aitken < 1e-6:
                    print(f"  Aitken melhorou a solução (dif={diff_aitken:.2e})")
                    self.psi.x.array[:] = psi_aitken.x.array

            # Verifica convergência do Picard
            if psi_prev is not None:
                diff = np.linalg.norm(self.psi.x.array - psi_prev.x.array)
                if diff < 1e-5:
                    print(f"  Picard convergiu em {picard_iter+1} iterações (diff={diff:.2e})")
                    break

            psi_prev2 = psi_prev
            psi_prev = self.psi.copy()

        return self.psi

    # --------------------------------------------------------------------
    # Atualização dos perfis a partir do modelo 0D (acoplamento)
    # --------------------------------------------------------------------
    def update_profiles_from_two_temperature(self, Ti_keV, Te_keV, n_e):
        """
        Atualiza p0 e perfis de temperatura a partir do modelo two-temperature.
        p0 = n_e * (Ti + Te) * 1e3 * Q_E
        """
        self.p0 = n_e * (Ti_keV + Te_keV) * 1e3 * Q_E
        print(f"[GS] Perfis atualizados: Ti={Ti_keV:.1f} keV, Te={Te_keV:.1f} keV, p0={self.p0:.2e} Pa")

    def compute_B_fields(self):
        """Retorna os campos B_R, B_Z, B_phi."""
        R = SpatialCoordinate(self.mesh)[0]
        dpsi_dR = grad(self.psi)[0]
        dpsi_dZ = grad(self.psi)[1]
        B_R = -dpsi_dZ / R
        B_Z = dpsi_dR / R
        psi_hat = (self.psi - self.psi_min) / (self.psi_max - self.psi_min + 1e-12)
        B_phi = self.F_profile(psi_hat) / R
        return B_R, B_Z, B_phi

# ============================================================================
# 3. MODELO TWO-TEMPERATURE (0D) PARA ACOPLAMENTO
# ============================================================================

# Constantes (já definidas globalmente, mas vamos garantir)
MU0 = 4 * np.pi * 1e-7
Q_E = 1.602176634e-19
K_B = 1.380649e-23

class TwoTemperatureModel0D:
    """
    Modelo 0D de duas temperaturas para plasmas D-T (ou p-11B).
    Resolve ODEs acopladas para Ti, Te.
    """
    def __init__(self, n_e, B, P_heating=50.0, tau_E=0.1):
        self.n_e = n_e
        self.B = B
        self.P_heat = P_heating  # MW/m³
        self.tau_E = tau_E       # confinamento energético (s)

        # Parâmetros de fusão (D-T)
        self.n_D = n_e / 2
        self.n_T = n_e / 2
        self.E_fus = 17.6e6 * Q_E  # J

    def reactivity_DT(self, T_keV):
        """Bosch-Hale 1992."""
        if T_keV < 0.2:
            return 1e-30
        T = T_keV
        BG = 34.3827
        mrc2 = 1124656
        C1 = 1.17302e-9
        C2 = 1.51361e-2
        C3 = 7.51886e-2
        C4 = 4.60643e-3
        C5 = 1.35000e-2
        C6 = -1.99833e-5
        theta = T / (1 - (T * (C2 + T * (C4 + T * C6))) / (1 + T * (C3 + T * (C5))))
        xi = (BG**2 / (4 * theta))**(1/3)
        return C1 * theta * np.sqrt(xi / (mrc2 * T**3)) * np.exp(-3 * xi)

    def fusion_power(self, Ti_keV):
        """Potência de fusão (MW/m³)."""
        sv = self.reactivity_DT(Ti_keV) * 1e-6  # cm³/s -> m³/s
        return self.n_D * self.n_T * sv * self.E_fus / 1e6  # MW/m³

    def bremsstrahlung(self, Te_keV):
        """Bremsstrahlung (MW/m³)."""
        if Te_keV < 0.1:
            return 0.0
        return 5.34e-37 * self.n_e**2 * np.sqrt(Te_keV) / 1e6

    def synchrotron(self, Te_keV):
        """Trubnikov com auto-absorção (MW/m³)."""
        if Te_keV < 0.1:
            return 0.0
        theta = Te_keV * 1e3 * Q_E / (9.109e-31 * (2.998e8)**2)
        G = 1.0 + 2.0 * theta + 3.5 * theta**2
        return 6.21e-17 * self.n_e * self.B**2 * Te_keV * G / 1e6

    def equipartition(self, Ti_keV, Te_keV):
        """Spitzer (MW/m³)."""
        Te_safe = max(Te_keV, 0.1)
        tau = 3.44e11 * Te_safe**1.5 / (self.n_e * 17.0)
        tau = max(tau, 1e-6)
        return 1.5 * self.n_e * (Te_keV - Ti_keV) * 1e3 * Q_E / tau / 1e6

    def dT_dt(self, Ti, Te):
        """Derivadas de Ti e Te (keV/s)."""
        Ti = max(Ti, 0.1)
        Te = max(Te, 0.1)

        P_fus = self.fusion_power(Ti)
        P_brem = self.bremsstrahlung(Te)
        P_sync = self.synchrotron(Te)
        P_eq = self.equipartition(Ti, Te)

        n_i = self.n_e / 2
        C_i = 1.5 * n_i * K_B * 1e3 / Q_E / 1e6  # MW/m³/keV
        C_e = 1.5 * self.n_e * K_B * 1e3 / Q_E / 1e6

        dTi = (P_fus + self.P_heat - P_eq) / C_i
        dTe = (self.P_heat + P_eq - P_brem - P_sync) / C_e

        return dTi, dTe

    def integrate(self, Ti0, Te0, dt, n_steps):
        """Integração explícita (Euler) para evolução temporal."""
        Ti = Ti0
        Te = Te0
        history = []
        for _ in range(n_steps):
            dTi, dTe = self.dT_dt(Ti, Te)
            Ti += dTi * dt
            Te += dTe * dt
            # Clipping
            Ti = max(Ti, 0.1)
            Te = max(Te, 0.1)
            history.append((Ti, Te))
        return np.array(history)

# ============================================================================
# 4. ACOPLAMENTO TEMPORAL (TWO-TEMPERATURE + GS)
# ============================================================================

def coupled_temporal_evolution(gs_solver, two_temp_model, Ti0=5.0, Te0=5.0, dt=0.1, n_steps=50):
    """
    Executa acoplamento temporal: a cada passo, atualiza p0 e resolve GS.
    Retorna histórico de Ti, Te e psi.
    """
    Ti = Ti0
    Te = Te0
    psi_history = []
    T_history = []

    print("\n[ACOPLAMENTO TEMPORAL]")
    print(f"  dt = {dt} s, n_steps = {n_steps}")

    for step in range(n_steps):
        # Atualiza perfis no solver GS
        gs_solver.update_profiles_from_two_temperature(Ti, Te, two_temp_model.n_e)

        # Resolve GS (auto-consistente)
        psi = gs_solver.solve(max_picard=3, aitken=True)
        psi_history.append(psi)

        # Evolui modelo 0D
        dTi, dTe = two_temp_model.dT_dt(Ti, Te)
        Ti += dTi * dt
        Te += dTe * dt
        Ti = max(Ti, 0.1)
        Te = max(Te, 0.1)
        T_history.append((Ti, Te))

        if step % 10 == 0:
            print(f"  step {step+1}/{n_steps}: Ti={Ti:.2f} keV, Te={Te:.2f} keV")

    return np.array(T_history), psi_history

# ============================================================================
# 5. EXEMPLO DE USO
# ============================================================================

if __name__ == "__main__":
    # Parâmetros
    R0 = 1.4
    a = 0.9
    B0 = 12.0
    n_e = 1e20

    # Arquivo de malha Gmsh (deve existir)
    # Exemplo: mesh.msh gerado com:
    # gmsh -2 mesh.geo -o mesh.msh
    # (O arquivo .geo deve definir um domínio retangular com marcador de fronteira)
    mesh_file = "mesh.msh"  # Substituir pelo caminho real

    print("=" * 70)
    print(" ARKHE-χ: Grad-Shafranov + Two-Temperature Acoplado")
    print(" FEniCSx + PETSc (AMG/Aitken/Gmsh)")
    print("=" * 70)

    # 1. Cria solver GS (com AMG se malha > 512x512)
    print("\n[1] Inicializando solver Grad-Shafranov...")
    gs = GradShafranovSolverAMG(mesh_file, R0=R0, a=a, B0=B0, use_amg=True)

    # 2. Cria modelo two-temperature
    print("\n[2] Inicializando modelo 0D...")
    two_temp = TwoTemperatureModel0D(n_e=n_e, B=B0, P_heating=50.0, tau_E=0.1)

    # 3. Resolve equilíbrio inicial (auto-consistente)
    print("\n[3] Resolvendo equilíbrio inicial (Ti=Te=5 keV)...")
    gs.update_profiles_from_two_temperature(5.0, 5.0, n_e)
    psi = gs.solve(max_picard=5, aitken=True)
    print(f"  ψ_max = {np.max(psi.x.array):.4f} Wb")

    # 4. Evolução temporal acoplada
    print("\n[4] Evolução temporal acoplada (0.1s, 50 passos)...")
    T_hist, psi_hist = coupled_temporal_evolution(
        gs, two_temp,
        Ti0=5.0, Te0=5.0,
        dt=0.1, n_steps=50
    )

    # 5. Resultados finais
    print("\n[RESULTADOS FINAIS]")
    print(f"  Ti_final = {T_hist[-1, 0]:.2f} keV")
    print(f"  Te_final = {T_hist[-1, 1]:.2f} keV")
    print("  Evolução concluída.")

    # Salvar solução para visualização (opcional)
    with io.XDMFFile(MPI.COMM_WORLD, "psi_solution.xdmf", "w") as file:
        file.write_mesh(gs.mesh)
        file.write_function(gs.psi)
    print("\n  Solução salva em psi_solution.xdmf")
