#!/bin/bash
# scripts/verify.sh — Executa todos os harnesses do SafeSail

set -e

echo "🔍 Verificando SafeSail com Kani..."

# Instalar Kani se necessário
if ! command -v kani &> /dev/null; then
    echo "📦 Instalando Kani..."
    cargo install kani-verifier
    kani setup
fi

cd arkhe-safe-sail

# Executar todos os harnesses
cargo kani -Z function-contracts --harness s1_pressure_bounded_by_one
cargo kani -Z function-contracts --harness s2_boundary_condition
cargo kani -Z function-contracts --harness s3_pressure_monotonic
cargo kani -Z function-contracts --harness s4_pressure_non_negative
cargo kani -Z function-contracts --harness s5_temporal_stability
cargo kani -Z function-contracts --harness s6_rate_reduction_never_increases_pressure
cargo kani -Z function-contracts --harness s7_zero_pressure_always_safe
cargo kani -Z function-contracts --harness s8_max_capacity_always_safe
cargo kani -Z function-contracts --harness s9_capacity_construction_no_overflow
cargo kani -Z function-contracts --harness s10_metrics_construction_no_panic
cargo kani -Z function-contracts --harness s11_multiple_metrics_invariants
cargo kani -Z function-contracts --harness check_pressure_contract

echo "✅ Todos os harnesses verificados com sucesso!"
