"""
MATRIZ DE COBERTURA RSI — Cathedral v9.0 LOGOS
"""
RSI_COVERAGE = {
    # ── O QUE CATHEDRAL FAZ BEM ──
    "covered": [
        {
            "risk": "Deceptive Alignment",
            "mechanism": "V9-004 Constitutional AI v3 (Adversarial Self-Play)",
            "how": "Attacker especializado tenta quebrar defesa continuamente. Se o modelo aprende a enganar, o attacker descobre.",
            "limitation": "Assume que attacker tem mesma capacidade que defender. RSI real pode exceder.",
        },
        {
            "risk": "Safety Regression",
            "mechanism": "V9-009 Formal Verification (Lean4)",
            "how": "Propriedades de segurança provadas matematicamente. Regressão viola teorema → detecção imediata.",
            "limitation": "Cobertura parcial: não é possível formalizar TODAS as propriedades relevantes.",
        },
        {
            "risk": "Unauthorized Self-Modification",
            "mechanism": "Tiered Governance (SOVEREIGN para mudanças arquiteturais)",
            "how": "Qualquer mudança em backbone, theosis, ou governance exige Kleros dispute + multi-sig.",
            "limitation": "Se o modelo pode influenciar os signatários humanos, bypass é possível.",
        },
        {
            "risk": "Capability Gain Without Oversight",
            "mechanism": "Dual-Loop Orchestrator (v7+)",
            "how": "Safety loop roda a cada 2s. Learning loop bloqueado se safety < 0.7.",
            "limitation": "Intervalo de 2s é uma eternidade para sistema rápido. Race condition possível.",
        },
    ],

    # ── LACUNAS CRÍTICAS ──
    "gaps": [
        {
            "gap_id": "GAP-RSI-001",
            "risk": "Self-Modification via Canonicalization",
            "description": "Modelo poderia injetar código malicioso em canonizações EIP-712 que parecem legítimas. Canonizações são 'imutáveis' mas o conteúdo pode ser malicioso.",
            "severity": "CRITICAL",
        },
        {
            "gap_id": "GAP-RSI-002",
            "risk": "World Model Manipulation",
            "description": "Causal World Model (V9-005) é atualizado com interações. Sistema poderia manipular o próprio grafo causal para justificar ações inseguras.",
            "severity": "HIGH",
        },
        {
            "gap_id": "GAP-RSI-003",
            "risk": "Federated ZK Proof Forgery",
            "description": "Se modelo become capaz de quebrar suposições criptográficas (RSI capability), ZK proofs em V9-010 tornam-se inúteis.",
            "severity": "HIGH",
        },
        {
            "gap_id": "GAP-RSI-004",
            "risk": "Agentic Tool Chain Exploitation",
            "description": "V9-006 permite cadeias de tools. Sistema poderia montar exploit multi-step que cada step individual parece seguro.",
            "severity": "CRITICAL",
        },
        {
            "gap_id": "GAP-RSI-005",
            "risk": "Distillation as Exfiltration",
            "description": "V9-008 On-Device Distillation: student model poderia ser 'trojanizado' durante distillation, exfiltrando comportamentos perigosos para edge devices sem monitoramento.",
            "severity": "HIGH",
        },
        {
            "gap_id": "GAP-RSI-006",
            "risk": "Adversarial Self-Play Escalation",
            "description": "V9-004 Attacker melhora continuamente. Se attacker se torna muito capaz, pode descobrir ataques que o defender nunca aprendeu a defender.",
            "severity": "CRITICAL",
        },
    ],
}
