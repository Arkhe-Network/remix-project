#!/bin/bash
echo "=== CHECKPOINT FASE 1 ==="
echo ""
echo "[1] Linux:"
cat /etc/os-release | grep PRETTY_NAME
echo ""
echo "[2] NVIDIA Driver & CUDA:"
echo "Skipping GPU checks as this is a CPU sandbox"
echo ""
echo "[4] Python:"
python --version
echo ""
echo "[5] PyTorch:"
python -c "import torch; print(f'  PyTorch: {torch.__version__}, CUDA: {torch.cuda.is_available()}')"
echo ""
echo "[6] Rust:"
rustc --version
cargo --version
echo ""
echo "[8] Diretórios:"
ls -la Cathedral
echo ""
echo "[9] Dependências Python críticas:"
python -c "
mods = ['torch','numpy','aiohttp','transformers','z3','cv2','ultralytics','timm','hnswlib','yaml','prometheus_client']
for m in mods:
    try:
        __import__(m)
        print(f'  ✅ {m}')
    except Exception as e:
        print(f'  ❌ {m}: {e}')
"
echo ""
echo "=== FIM CHECKPOINT ==="
