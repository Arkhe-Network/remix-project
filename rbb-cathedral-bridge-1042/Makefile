# Makefile completo para o Emulador Quântico Cathedral
# Compila, executa, gera vetores, e prepara para testes Foundry

CXX = g++
CXXFLAGS = -std=c++17 -O3 -Wall -Wextra -pthread
LDFLAGS = -lssl -lcrypto -lpthread

# Alvos
TARGET = cathedral_emulator
PYTHON_SCRIPT = cathedral_emulator.py
TEST_CONTRACT = CathedralQuantumEmulatorTest.t.sol

# Diretórios
SRC_DIR = .
BUILD_DIR = build
OUTPUT_DIR = output

# Arquivos
CPP_SRC = $(SRC_DIR)/cathedral_emulator.cpp

# Regras
.PHONY: all clean compile run test vectors foundry-test deploy

all: compile

$(BUILD_DIR):
    mkdir -p $(BUILD_DIR)

$(OUTPUT_DIR):
    mkdir -p $(OUTPUT_DIR)

compile: $(BUILD_DIR) $(CPP_SRC)
    $(CXX) $(CXXFLAGS) -o $(BUILD_DIR)/$(TARGET) $(CPP_SRC) $(LDFLAGS)
    @echo "✓ Compilação concluída: $(BUILD_DIR)/$(TARGET)"

run: compile
    @echo "=== Executando Emulador Quântico ==="
    $(BUILD_DIR)/$(TARGET) | tee $(OUTPUT_DIR)/emulator_output.txt
    @echo "✓ Saída salva em $(OUTPUT_DIR)/emulator_output.txt"

vectors: run
    @echo "=== Gerando Vetores de Teste ==="
    python3 $(PYTHON_SCRIPT) > $(OUTPUT_DIR)/test_vectors.json
    @echo "✓ Vetores salvos em $(OUTPUT_DIR)/test_vectors.json"

# Testes Foundry (requer Foundry instalado)
foundry-test: vectors
    @echo "=== Executando Testes Foundry ==="
    forge test --match-contract CathedralQuantumEmulatorTest --gas-report -vvv
    @echo "✓ Testes concluídos"

# Deploy na testnet (requer variáveis de ambiente)
deploy:
    @echo "=== Deploy na RBB Chain Testnet ==="
    forge script DeployQuantumOracle --rpc-url $(RBB_RPC) --private-key $(PRIVATE_KEY) --broadcast
    @echo "✓ Deploy concluído"

# Limpeza
clean:
    rm -rf $(BUILD_DIR) $(OUTPUT_DIR)
    @echo "✓ Limpo"

# Ajuda
help:
    @echo "Alvos disponíveis:"
    @echo "  compile      - Compila o emulador C++"
    @echo "  run          - Executa o emulador"
    @echo "  vectors      - Gera vetores de teste (Python)"
    @echo "  foundry-test - Executa testes Foundry"
    @echo "  deploy       - Deploy na RBB Chain testnet"
    @echo "  clean        - Limpa arquivos gerados"
