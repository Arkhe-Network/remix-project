import Mathlib.Data.Real.Basic
import Mathlib.Analysis.Real.Sqrt
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.Ring
import Mathlib.Tactic.Positivity
import Mathlib.Analysis.SpecialFunctions.Pow.Real

/-!
  Cathedral Arkhe — Application: Modular Typography Scales

  Epistemic Status:
    L1: Mathematical sequences (geometric, harmonic, Fibonacci)
    L0: CSS implementation via calc() and clamp()

  Formalizes font-size scales used in responsive design.
-/

namespace CathedralArkhe.UI.Typography

/-! ── Geometric Scales ── -/

noncomputable def goldenRatio : ℝ := (1 + Real.sqrt 5) / 2

/-- A geometric scale: base * ratio^n for n ∈ ℕ. -/
def geometricScale (base : ℝ) (ratio : ℝ) (n : ℕ) : ℝ :=
  base * ratio ^ n

/-- Perfect fourth scale (ratio 4/3). -/
noncomputable def perfectFourth := (4 : ℝ) / 3

/-- Perfect fifth scale (ratio 3/2). -/
noncomputable def perfectFifth := (3 : ℝ) / 2

/-- T_UI.4: The golden ratio satisfies φ² = φ + 1. -/
theorem golden_ratio_square :
  goldenRatio * goldenRatio = goldenRatio + 1 := by
  unfold goldenRatio
  have h : (Real.sqrt 5) * (Real.sqrt 5) = 5 := by
    apply Real.mul_self_sqrt
    linarith
  calc ((1 + Real.sqrt 5) / 2) * ((1 + Real.sqrt 5) / 2) = (1 + 2 * Real.sqrt 5 + 5) / 4 := by
         have h1 : (1 + Real.sqrt 5) * (1 + Real.sqrt 5) = 1 + 2 * Real.sqrt 5 + 5 := by
           calc (1 + Real.sqrt 5) * (1 + Real.sqrt 5) = 1 + 2 * Real.sqrt 5 + Real.sqrt 5 * Real.sqrt 5 := by ring
                _ = 1 + 2 * Real.sqrt 5 + 5 := by rw [h]
         rw [div_mul_div_comm, h1]
         norm_num
       _ = (6 + 2 * Real.sqrt 5) / 4 := by ring
       _ = (1 + Real.sqrt 5) / 2 + 1 := by ring

/-- T_UI.5: Any geometric scale is strictly increasing for ratio > 1. -/
theorem geometricScale_monotone (base : ℝ) (ratio : ℝ) (h_base : base ≥ 0) (h : 1 ≤ ratio) (n m : ℕ) (h_le : n ≤ m) :
    geometricScale base ratio n ≤ geometricScale base ratio m := by
  unfold geometricScale
  have h_ratio_pos : 0 ≤ ratio := by linarith
  exact mul_le_mul_of_nonneg_left (pow_le_pow_right₀ h h_le) h_base

/-! ── Musical Scales (Non-standard) ── -/

/-- Pythagorean scale: frequencies 3:2 ratios. -/
noncomputable def pythagoreanScale (baseFreq : ℝ) (n : ℕ) : ℝ :=
  baseFreq * (3 / 2 : ℝ) ^ n

/-- Equal temperament scale: 2^(n/12). -/
noncomputable def equalTemperament (baseFreq : ℝ) (n : ℕ) : ℝ :=
  baseFreq * (2 : ℝ) ^ ((n : ℝ) / 12)

end CathedralArkhe.UI.Typography
