#!/bin/sh
# Script de build e execução do Emulador Quântico Cathedral
# Uso: ./build_and_run.sh [compile|run|vectors|test|all]

set -e

BUILD_DIR="build"
OUTPUT_DIR="output"
TARGET="cathedral_emulator"

# Cores
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
NC="\033[0m" # No Color

function log() {
    echo -e "${GREEN}[BUILD]${NC} $1"
}

function warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

function erro() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Verificar dependências
function check_deps() {
    log "Verificando dependências..."

    # g++
    if ! command -v g++ &> /dev/null; then
        erro "g++ não encontrado. Instale: sudo apt-get install build-essential"
    fi

    # OpenSSL
    if ! pkg-config --exists openssl; then
        erro "OpenSSL não encontrado. Instale: sudo apt-get install libssl-dev"
    fi

    # Python3
    if ! command -v python3 &> /dev/null; then
        erro "python3 não encontrado. Instale: sudo apt-get install python3"
    fi

    # Foundry (opcional)
    if ! command -v forge &> /dev/null; then
        warn "Foundry não encontrado. Testes Solidity não serão executados."
        warn "Instale: curl -L https://foundry.paradigm.xyz | sh"
    fi

    log "Dependências OK"
}

# Compilar
function compile() {
    log "Compilando emulador C++..."
    mkdir -p $BUILD_DIR

    g++ -std=c++17 -O3 -Wall -Wextra -pthread \
        -o $BUILD_DIR/$TARGET \
        cathedral_emulator.cpp \
        -lssl -lcrypto -lpthread

    log "✓ Compilação concluída: $BUILD_DIR/$TARGET"
}

# Executar
function run() {
    if [ ! -f "$BUILD_DIR/$TARGET" ]; then
        compile
    fi

    log "Executando emulador..."
    mkdir -p $OUTPUT_DIR

    $BUILD_DIR/$TARGET | tee $OUTPUT_DIR/emulator_output.txt

    log "✓ Saída salva em $OUTPUT_DIR/emulator_output.txt"
}

# Gerar vetores
function vectors() {
    log "Gerando vetores de teste (Python)..."

    python3 cathedral_emulator.py > $OUTPUT_DIR/test_vectors.json

    log "✓ Vetores salvos em $OUTPUT_DIR/test_vectors.json"

    # Extrair valores para substituição nos testes Foundry
    if [ -f "$OUTPUT_DIR/test_vectors.json" ]; then
        log "Analisando vetores..."
        python3 -c "
import json
with open("$OUTPUT_DIR/test_vectors.json") as f:
    data = json.load(f)
    print(f"Vetores gerados: {len(data)}")
    if data:
        print(f"Mensagem: {data[0][\"message\"][:64]}...")
        print(f"Signature: {data[0][\"signature\"][:64]}...")
        print(f"PublicKey: {data[0][\"publicKeyRoot\"][:64]}...")
"
    fi
}

# Testes Foundry
function foundry_test() {
    if ! command -v forge &> /dev/null; then
        erro "Foundry não instalado. Não é possível executar testes Solidity."
    fi

    log "Executando testes Foundry..."

    # Atualizar placeholders nos testes se vetores existirem
    if [ -f "$OUTPUT_DIR/test_vectors.json" ]; then
        log "Atualizando placeholders nos testes..."
        python3 scripts/update_test_vectors.py \
            $OUTPUT_DIR/test_vectors.json \
            CathedralQuantumEmulatorTest.t.sol
    fi

    forge test --match-contract CathedralQuantumEmulatorTest --gas-report -vvv

    log "✓ Testes concluídos"
}

# Deploy
function deploy() {
    if [ -z "$RBB_RPC" ]; then
        erro "Variável RBB_RPC não definida. Exemplo: export RBB_RPC=https://testnet.rbbchain.com/rpc"
    fi

    if [ -z "$PRIVATE_KEY" ]; then
        erro "Variável PRIVATE_KEY não definida."
    fi

    log "Deploy na RBB Chain testnet..."

    forge script DeployQuantumOracle \
        --rpc-url $RBB_RPC \
        --private-key $PRIVATE_KEY \
        --broadcast \
        --verify

    log "✓ Deploy concluído"
}

# Limpar
function clean() {
    log "Limpando..."
    rm -rf $BUILD_DIR $OUTPUT_DIR
    log "✓ Limpo"
}

# Main
case "${1:-all}" in
    compile)
        check_deps
        compile
        ;;
    run)
        check_deps
        run
        ;;
    vectors)
        check_deps
        vectors
        ;;
    test)
        check_deps
        foundry_test
        ;;
    deploy)
        check_deps
        deploy
        ;;
    all)
        check_deps
        compile
        run
        vectors
        foundry_test
        ;;
    clean)
        clean
        ;;
    help)
        echo "Uso: $0 [compile|run|vectors|test|deploy|all|clean]"
        echo ""
        echo "Alvos:"
        echo "  compile  - Compila o emulador C++"
        echo "  run      - Executa o emulador"
        echo "  vectors  - Gera vetores de teste"
        echo "  test     - Executa testes Foundry"
        echo "  deploy   - Deploy na RBB Chain testnet"
        echo "  all      - Executa tudo (padrão)"
        echo "  clean    - Limpa arquivos gerados"
        ;;
    *)
        erro "Alvo desconhecido: $1. Use "help" para ver opções."
        ;;
esac
