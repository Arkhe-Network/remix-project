# TopoMAS-PoUW v1.1 — Correções Críticas

## 📋 Resumo das Correções

| Problema v1.0 | Correção v1.1 | Arquivo |
|---------------|---------------|---------|
| **T1** — Bug de indexação batch no planner | `action_vec[:, :n_actions]` com batch handling correto | `agents/latent_planner_v11.py` |
| **T2** — "Hebbian" falso (era RNN padrão) | Renomeado para **Recurrent Latent Planner**, arquitetura honesta | `agents/latent_planner_v11.py` |
| **T3** — FNO 1D sem PBC, energia como média | **FNO 3D** com grid de densidade atômica + PBC circular, energia como **integral extensiva** | `physicofm/neural_operator_3d.py` |
| **T4** — EWC: Fisher nos dados NOVOS | Fisher computada nos dados de **REFERÊNCIA** (tarefa anterior) | `continual/continual_learner_v11.py` |
| **T5** — Fisher normalizada por batches | Normalizada por **número de amostras** (`len(dataset)`) | `continual/continual_learner_v11.py` |
| **H1** — Energia intensiva (média) | Energia **extensiva** (soma × dV) | `physicofm/neural_operator_3d.py` |
| **H3** — Contexto com `F.pad` (125 zeros) | Projeção real via `nn.Linear(6, state_dim)` | `agents/latent_planner_v11.py` |

---

## 🗂️ Estrutura

```
src/topomas/
├── physicofm/
│   └── neural_operator_3d.py      # FNO 3D + PBC + AtomicDensityGrid
├── agents/
│   └── latent_planner_v11.py      # Recurrent Latent Planner (batch-safe)
├── continual/
│   └── continual_learner_v11.py   # EWC corrigido + Replay Buffer
└── tests/
    └── test_suite_v11.py          # 7 testes de regressão
```
