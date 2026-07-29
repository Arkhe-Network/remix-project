#!/bin/bash
# Cathedral ARKHE v28.3 — Compilar circuito LlamaZip ZK e extrair Image ID
# Selo: CATHEDRAL-ARKHE-v28.3-LLAMAZIP-ZK-BUILD-2026-06-16

set -euo pipefail

CIRCUIT_DIR="cathedral-arkhe-v28.3/zk-circuits/llama-zip-verify"
GUEST_DIR="$CIRCUIT_DIR/methods/guest"

echo "🔧 Compilando guest RISC Zero LlamaZip..."
cd "$GUEST_DIR"
cargo risczero build --release

BINARY="target/riscv-guest/riscv32im-risc0-zkvm-elf/release/llama-zip-verify-guest"
if [ ! -f "$BINARY" ]; then
    echo "❌ Binário do guest não encontrado em $BINARY"
    exit 1
fi

echo "✅ Guest compilado com sucesso"
echo "📦 Binário: $(realpath "$BINARY")"

# Extrair Image ID
IMAGE_ID=$(jq -r '.image_id' "${BINARY}.json")
echo "🆔 SPHINCS_IMAGE_ID=$IMAGE_ID"
echo "$IMAGE_ID" > ../../../llama_zip_image_id.txt
echo "💾 Image ID salvo em llama_zip_image_id.txt"
