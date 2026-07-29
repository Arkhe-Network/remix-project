"""
Cathedral ARKHE v17.0 - Provenance & Cryptographic Signature (Ato 2)
Garante que o modelo possui identidade verificável e imutável.
"""
import os
import json
import hashlib
from pathlib import Path
from cryptography.hazmat.primitives.asymmetric import ed25519

class CathedralProvenance:
    def __init__(self, keys_dir="keys"):
        self.keys_dir = Path(keys_dir)
        self.keys_dir.mkdir(parents=True, exist_ok=True)
        self.priv_key_path = self.keys_dir / "cathedral_ed25519_priv.pem"
        self.pub_key_path = self.keys_dir / "cathedral_ed25519_pub.pem"
        self._load_or_generate_keys()

    def _load_or_generate_keys(self):
        if self.priv_key_path.exists():
            with open(self.priv_key_path, "rb") as f:
                self.private_key = ed25519.Ed25519PrivateKey.from_private_bytes(f.read())
            with open(self.pub_key_path, "rb") as f:
                self.public_key = ed25519.Ed25519PublicKey.from_public_bytes(f.read())
        else:
            self.private_key = ed25519.Ed25519PrivateKey.generate()
            self.public_key = self.private_key.public_key()
            with open(self.priv_key_path, "wb") as f:
                f.write(self.private_key.private_bytes_raw())
            with open(self.pub_key_path, "wb") as f:
                f.write(self.public_key.public_bytes_raw())

    def sign_model_weights(self, weights_path: str) -> str:
        """Calcula hash dos pesos e gera assinatura."""
        print(f"[Provenance] Calculando hash SHA-256 de {weights_path}...")
        sha256 = hashlib.sha256()
        with open(weights_path, "rb") as f:
            while chunk := f.read(8192):
                sha256.update(chunk)
        weights_hash = sha256.hexdigest()

        signature = self.private_key.sign(weights_hash.encode())
        sig_hex = signature.hex()
        print(f"[Provenance] Hash: {weights_hash[:16]}... Assinado com sucesso.")
        return weights_hash, sig_hex

    def inject_gguf_manifest(self, gguf_path: str, author: str = "Cathedral-ARKHE-v17"):
        """Injeta os metadados de proveniência em um arquivo GGUF."""
        try:
            import gguf
        except ImportError:
            print("[Provenance] Biblioteca 'gguf' não instalada. Use: pip install gguf")
            return

        weights_hash, sig_hex = self.sign_model_weights(gguf_path)

        # Lê o arquivo existente
        reader = gguf.GGUFReader(gguf_path)
        arch = reader.fields.get("general.architecture")

        # Prepara o manifesto para injetar (requer re-empacotamento GGUF)
        # Nota: A biblioteca gguf python foca em leitura. Para escrita completa,
        # usa-se a ferramenta 'gguf-py' ou o script C original.
        # Aqui geramos o JSON do manifesto para ser anexado via ferramenta externa.
        manifest = {
            "general.provenance": {
                "author": author,
                "version": "17.0",
                "weights_hash_sha256": weights_hash,
                "signature_ed25519": sig_hex,
                "pub_key_path": str(self.pub_key_path),
                "lineage": "Nex N2 Pro + Qwen 3.5 (Base) -> Cathedral (Fine-tuned/Erased)"
            }
        }
        manifest_path = Path(gguf_path).parent / "cathedral_manifest.json"
        with open(manifest_path, "w") as f:
            json.dump(manifest, f, indent=2)
        print(f"[Provenance] Manifesto gerado em {manifest_path}. Anexe ao GGUF.")

    def verify_model(self, weights_path: str, known_hash: str, signature_hex: str) -> bool:
        """Verifica se o modelo foi alterado."""
        sha256 = hashlib.sha256()
        with open(weights_path, "rb") as f:
            while chunk := f.read(8192):
                sha256.update(chunk)
        current_hash = sha256.hexdigest()

        try:
            self.public_key.verify(bytes.fromhex(signature_hex), current_hash.encode())
            return current_hash == known_hash
        except Exception:
            return False
