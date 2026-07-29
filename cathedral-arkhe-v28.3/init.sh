#!/bin/bash
# Script para inicializar a stack

echo "🚀 Iniciando inicialização da stack Cathedral ARKHE v28.3..."

# 1. Verificar dependências
echo "📦 Verificando dependências..."
if ! command -v docker &> /dev/null; then
    echo "Docker não encontrado. Abortando."
    return 1 2>/dev/null || true
fi
if ! command -v docker-compose &> /dev/null; then
    echo "docker-compose não encontrado. Abortando."
    return 1 2>/dev/null || true
fi

# 2. Construir imagens
echo "🏗️ Construindo imagens Docker..."
cd runtime || return 1 2>/dev/null || true
docker-compose build

# 3. Subir infraestrutura base (banco de dados, cache, etc)
echo "🌐 Subindo infraestrutura base (TemporalChain, Vector DB, Redis, Jaeger)..."
docker-compose up -d temporal-chain vector-db redis jaeger

# 4. Aguardar serviços estarem prontos
echo "⏳ Aguardando serviços inicializarem..."
sleep 10

# 5. Subir servidor LLM
echo "🧠 Subindo servidor de inferência LLM..."
docker-compose up -d llm-server

# 6. Aguardar LLM
echo "⏳ Aguardando LLM Server (pode demorar)..."
sleep 15

# 7. Iniciar agente runtime
echo "🤖 Iniciando Agente Orchestrator..."
docker-compose up -d agent-runtime

echo "✅ Inicialização completa! Verifique os logs com: docker-compose logs -f"
