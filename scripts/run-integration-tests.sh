#!/bin/bash
# scripts/run-integration-tests.sh

echo "🏛️  Executando testes de integração Taproot Assets"

# Verifica se o ambiente está rodando
if ! docker ps | grep -q tapd-alice; then
    echo "❌ Ambiente não está rodando. Execute ./scripts/setup-regtest.sh primeiro."
    exit 1
fi

# Executa os testes
cargo test -p cathedral-taproot-bridge --test integration -- --nocapture --test-threads=1

echo "✅ Testes de integração concluídos"
