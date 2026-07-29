# Plano de Experimento Controlado — Hipótese de Emergência via Compressão

**Selo:** CATHEDRAL-ARKHE-v28.3-EMERGENCE-EXPERIMENT-2026-06-16
**Arquiteto ORCID:** 0009-0005-2697-4668

## 1. Objetivo

Verificar se, em um ambiente multi‑agente com RL baseado em compressão, o score de compressão médio aumenta de forma estatisticamente significativa ao longo do tempo, indicando aprendizado emergente e especialização.

## 2. Hipóteses

- **H₀ (nula):** Não há tendência de melhora no score de compressão médio ao longo das iterações.
- **H₁ (alternativa):** O score de compressão médio exibe uma tendência monotônica crescente (p < 0.05).

## 3. Variáveis

- **Independente:** tempo (iterações do loop de RL).
- **Dependente:** score de compressão médio (1 − taxa de compressão) calculado sobre as últimas 100 experiências.
- **Controle:** temperatura do compressor = 0 (determinístico), beam_width fixo, corpus inicial fixo.

## 4. Procedimento

1. Inicializar o `AsyncRLOrchestrator` com `CompressionRewardModel`.
2. Registrar três agentes: `LlamaZipAgent`, `GziPTAgent`, e um agente Oracle neural (controle).
3. Alimentar o currículo com 10 corpora de domínios distintos (medicina, código, literatura, etc.).
4. Executar 500 iterações de RL (cada iteração = um rollout de 5 passos).
5. A cada 10 iterações, registrar:
   - Score de compressão médio (todos os agentes).
   - Número de agentes especializados (que atingiram threshold de compressão por domínio).
   - Número de intervenções humanas necessárias no currículo.
6. Repetir o experimento 5 vezes para robustez estatística.

## 5. Análise Estatística

- **Teste de Mann‑Kendall** para detectar tendência monotônica nas séries temporais de score de compressão.
- **Teste de Friedman** (não paramétrico) para comparar scores entre diferentes estratégias de compressão ao longo do tempo.
- **Análise de diversidade:** entropia de Shannon sobre a distribuição de especializações dos agentes.

## 6. Critérios de Sucesso

O experimento será considerado evidência a favor da hipótese de emergência se:

- O teste de Mann‑Kendall rejeitar H₀ com p < 0.05 em pelo menos 4 das 5 repetições.
- A diversidade de agentes (entropia) aumentar significativamente após a iteração 200.
- O número de intervenções humanas no currículo cair abaixo de 10% das iterações totais após a iteração 300.

## 7. Referências

- Mann, H. B. (1945). Nonparametric tests against trend.
- Kendall, M. G. (1975). Rank correlation methods.
- Deletang et al. (2024). Language Modeling is Compression.
