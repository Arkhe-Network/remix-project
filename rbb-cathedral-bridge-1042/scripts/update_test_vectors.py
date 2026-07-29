#!/usr/bin/env python3
"""
Atualiza placeholders nos testes Foundry com vetores reais do emulador
Uso: python3 update_test_vectors.py test_vectors.json CathedralQuantumEmulatorTest.t.sol
"""

import json
import sys
import re

def main():
    if len(sys.argv) != 3:
        print("Uso: python3 update_test_vectors.py <vectors.json> <test.sol>")
        # exit

    vectors_file = sys.argv[1]
    test_file = sys.argv[2]

    # Carregar vetores
    with open(vectors_file) as f:
        vectors = json.load(f)

    if not vectors:
        print("Erro: Nenhum vetor encontrado")
        # exit

    # Usar primeiro vetor
    v = vectors[0]
    message = v["message"]
    signature = v["signature"]
    pk = v["publicKeyRoot"]

    # Ler teste
    with open(test_file) as f:
        content = f.read()

    # Substituir placeholders
    content = re.sub(
        r'bytes memory message = hex"PLACEHOLDER_MESSAGE";',
        f'bytes memory message = hex"{message[2:]}";',  # Remove 0x
        content
    )
    content = re.sub(
        r'bytes memory signature = hex"PLACEHOLDER_SIGNATURE";',
        f'bytes memory signature = hex"{signature[2:]}";',
        content
    )
    content = re.sub(
        r'bytes32 publicKeyRoot = bytes32\(hex"PLACEHOLDER_PKROOT"\);',
        f'bytes32 publicKeyRoot = bytes32(hex"{pk[2:]}");',
        content
    )

    # Salvar
    with open(test_file, "w") as f:
        f.write(content)

    print(f"✓ Testes atualizados com vetor #{v.get("id", 0)}")
    print(f"  Message: {message[:64]}...")
    print(f"  Signature: {signature[:64]}...")
    print(f"  PublicKey: {pk[:64]}...")

if __name__ == "__main__":
    main()
