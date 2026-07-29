# 🏛️ ORCHIDEAS — Mapeamento de Pilares para a Arquitetura Arkhe OS

**Data:** 2026-06-25
**Versão:** 1.0.0

---

## 1. Autonomia

**Definição ORCHIDEAS:** Níveis de autonomia dos agentes e limites de atuação.

**Aplicação Arkhe OS:**
- `arkhe-agents/src/rlm/harness.rs` → `RlmConfig::max_iterations`, `RlmConfig::consent_required`
- `arkhe-agents/src/safety/validator.rs` → `SafetyValidator::is_code_safe`
- `arkhe-agents/src/safety/consent.rs` → `ConsentManager::request_consent`

**Verificação:** `ConsentManager` nunca deve ser bypassado sem rastreamento no WormGraph.

---

## 2. Identity & Intent

**Definição ORCHIDEAS:** Tokens de intenção atestados (DIDs) para cada ação.

**Aplicação Arkhe OS:**
- `ArkheKernel/src/state.rs` → `Principal`, `EntityId`, `CapabilityMask`
- `ArkheKernel/src/action.rs` → `ActionCompute` trait com `submit()`
- `ArkheKernel/src/wal.rs` → Assinatura híbrida (ML-DSA-65 + Ed25519)

**Verificação:** `Principal` DEVE estar presente em cada ação registrada no WAL.

---

## 3. Data & Memory Governance

**Definição ORCHIDEAS:** Controle granular sobre o que é lembrado, compartilhado e por quanto tempo.

**Aplicação Arkhe OS:**
- `arkhe-core/src/string_safe.rs` → `SafeString` com verificação Kani
- `arkhe-neural/src/episodic_memory.rs` → `EpisodicWorkingMemory` com TTL
- `ArkheKernel/src/wal.rs` → Cadeia de hash BLAKE3 sobre registros postcard-canonical

**Verificação:** TTL DEVE ser respeitado; dados expirados NÃO DEVEM ser recuperáveis.

---

## 4. Context

**Definição ORCHIDEAS:** Isolamento estrutural entre fontes confiáveis e não confiáveis.

**Aplicação Arkhe OS:**
- `arkhe-agents/src/rlm/context.rs` → `RlmContext` com máximo de 200 entradas
- `arkhe-agents/src/rlm/sandbox.rs` → `SandboxManager` com módulos permitidos
- `arkhe-llm/src/engine.rs` → `RemoteEngine` vs `LocalEngine`

**Verificação:** Contexto NÃO DEVE ser contaminado por entrada não confiável.

---

## 5. Runtime

**Definição ORCHIDEAS:** Execução monitorada e isolada de agentes em Capsules.

**Aplicação Arkhe OS:**
- `ArkheKernel/src/lib.rs` → `step()` e `submit()` com isolamento
- `arkhe-agents/src/rlm/harness.rs` → `RlmHarness::run()` com loop controlado
- `arkhe-forge/src/wasm.rs` → WASM sandbox com Kani-verified host-fn boundary

**Verificação:** Runtime DEVE monitorar consumo de recursos e isolar agentes.

---

## 6. Human Oversight & Override

**Definição ORCHIDEAS:** Mecanismos robustos para intervenção humana.

**Aplicação Arkhe OS:**
- `arkhe-agents/src/safety/consent.rs` → `ConsentManager`
- `arkhe-agents/src/rlm/harness.rs` → `RlmAction::WaitForHuman`
- `arkhe-agents/src/safety/validator.rs` → `SafetyValidator` (hard stops)

**Verificação:** Override humano DEVE ser possível e registrado no WormGraph.

---

## 7. Observability

**Definição ORCHIDEAS:** Visibilidade completa do sistema.

**Aplicação Arkhe OS:**
- `ArkheKernel/src/view.rs` → `InstanceView` read API
- `ArkheKernel/src/observer.rs` → Observer pipeline com selagem
- `arkhe-core/src/metrics.rs` → Métricas de performance e segurança

**Verificação:** Observadores NÃO DEVEM ter efeitos colaterais; logs DEVEM ser imutáveis.

---

## 8. Eval/Environment/Ecosystem

**Definição ORCHIDEAS:** Avaliação contínua da segurança do sistema e de seus componentes.

**Aplicação Arkhe OS:**
- `.github/workflows/legacy-audit.yml` → Auditoria semanal
- `.github/workflows/ai-audit-weekly.yml` → Auditoria IA semanal
- `.github/workflows/tla-check.yml` → Verificação formal TLA+
- `.github/workflows/kani-proofs.yml` → Verificação formal Kani

**Verificação:** CI/CD DEVE falhar se qualquer avaliação de segurança falhar.

---

## 9. Scalability

**Definição ORCHIDEAS:** Crescimento seguro da arquitetura e da base de agentes.

**Aplicação Arkhe OS:**
- `ArkheKernel/src/state.rs` → Arquitetura em DAG com camadas unidirecionais
- `arkhe-neural/src/episodic_memory.rs` → `capacity` limitado e LRU eviction
- `arkhe-agents/src/rlm/context.rs` → `MAX_ENTRIES` limitado

**Verificação:** Adição de novos agentes NÃO DEVE degradar segurança da camada base.

---

## 📊 Matriz de Cobertura

| Pilar | Implementado | CI Verificado | Kani Proof | TLA+ |
|-------|--------------|---------------|------------|------|
| 1. Autonomia | ✅ | ✅ | 🔴 | 🔴 |
| 2. Identity & Intent | ✅ | ✅ | ✅ | ✅ |
| 3. Data & Memory Governance | ✅ | ✅ | ✅ | ✅ |
| 4. Context | ✅ | ✅ | 🔴 | 🔴 |
| 5. Runtime | ✅ | ✅ | ✅ | 🔴 |
| 6. Human Oversight | 🔴 | 🔴 | 🔴 | 🔴 |
| 7. Observability | ✅ | 🔴 | 🔴 | 🔴 |
| 8. Eval/Environment | ✅ | ✅ | ✅ | ✅ |
| 9. Scalability | ✅ | 🔴 | 🔴 | 🔴 |

**Selo:** ARKHE-ORCHIDEAS-MAPPING-v1.0.0-2026-06-25