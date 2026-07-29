import os
import sys
import json
import hashlib
from eth_tester import EthereumTester, PyEVMBackend
from web3 import Web3, EthereumTesterProvider
from solcx import compile_standard, install_solc

install_solc("0.8.28")

import sys
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../../rbb-cathedral-bridge-1042/scripts')))
import sphincs_c13

backend = PyEVMBackend()
tester = EthereumTester(backend=backend)
w3 = Web3(EthereumTesterProvider(tester))

accounts = w3.eth.accounts
deployer = accounts[0]
agent = accounts[1]

def compile_contracts():
    with open('enter-cathedral-bridge/contracts/QuantumTimestampOracle.sol', 'r') as f:
        oracle_source = f.read()
    with open('rbb-cathedral-bridge-1042/contracts/CathedralSPHINCSVerifier.sol', 'r') as f:
        verifier_source = f.read()
    with open('enter-cathedral-bridge/contracts/EnterEvidenceAnchor.sol', 'r') as f:
        anchor_source = f.read()

    compiled_sol = compile_standard({
        "language": "Solidity",
        "sources": {
            "QuantumTimestampOracle.sol": {"content": oracle_source},
            "CathedralSPHINCSVerifier.sol": {"content": verifier_source},
            "EnterEvidenceAnchor.sol": {"content": anchor_source}
        },
        "settings": {
            "viaIR": True,
            "optimizer": {
                "enabled": True,
                "runs": 200
            },
            "outputSelection": {
                "*": {
                    "*": ["abi", "evm.bytecode"]
                }
            }
        }
    }, solc_version="0.8.28", allow_paths=".")

    return compiled_sol

def deploy_mock_oracle(compiled_sol, pub_key_root):
    mock_oracle_src = """
    // SPDX-License-Identifier: Apache-2.0
    pragma solidity ^0.8.28;
    contract MockOracle {
        uint64 public tick;
        bytes32 public pkRoot;

        constructor(bytes32 _pkRoot) {
            pkRoot = _pkRoot;
            tick = 1;
        }

        function getTimestamp() external view returns (uint64, bytes memory) {
            return (tick, new bytes(0));
        }
        function publicKeyRoot() external view returns (bytes32) {
            return pkRoot;
        }
        function setTick(uint64 _tick) external {
            tick = _tick;
        }
    }
    """
    compiled_mock = compile_standard({
        "language": "Solidity",
        "sources": {
            "MockOracle.sol": {"content": mock_oracle_src}
        },
        "settings": {
            "outputSelection": {
                "*": {
                    "*": ["abi", "evm.bytecode"]
                }
            }
        }
    }, solc_version="0.8.28")

    contract_interface = compiled_mock['contracts']['MockOracle.sol']['MockOracle']
    MockOracle = w3.eth.contract(abi=contract_interface['abi'], bytecode=contract_interface['evm']['bytecode']['object'])

    tx_hash = MockOracle.constructor(pub_key_root).transact({'from': deployer})
    tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

    return w3.eth.contract(address=tx_receipt.contractAddress, abi=contract_interface['abi'])

def deploy_contracts(compiled_sol, oracle_address):
    verifier_interface = compiled_sol['contracts']['CathedralSPHINCSVerifier.sol']['CathedralSPHINCSVerifier']
    Verifier = w3.eth.contract(abi=verifier_interface['abi'], bytecode=verifier_interface['evm']['bytecode']['object'])
    tx_hash = Verifier.constructor().transact({'from': deployer})
    tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
    verifier_address = tx_receipt.contractAddress

    anchor_interface = compiled_sol['contracts']['EnterEvidenceAnchor.sol']['EnterEvidenceAnchor']
    Anchor = w3.eth.contract(abi=anchor_interface['abi'], bytecode=anchor_interface['evm']['bytecode']['object'])
    tx_hash = Anchor.constructor(verifier_address, oracle_address, agent).transact({'from': deployer})
    tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
    anchor_address = tx_receipt.contractAddress

    return w3.eth.contract(address=anchor_address, abi=anchor_interface['abi'])

def merkle_root(leaves):
    level = leaves
    while len(level) > 1:
        next_level = []
        for i in range(0, len(level), 2):
            if i + 1 < len(level):
                left = level[i]
                right = level[i+1]
                if left < right:
                    next_level.append(Web3.keccak(left + right))
                else:
                    next_level.append(Web3.keccak(right + left))
            else:
                next_level.append(level[i])
        level = next_level
    return level[0]

def merkle_proof(leaves, index):
    level = leaves
    idx = index
    proof = []
    while len(level) > 1:
        next_level = []
        for i in range(0, len(level), 2):
            if i + 1 < len(level):
                left = level[i]
                right = level[i+1]

                if i == idx or i + 1 == idx:
                    sibling_idx = i + 1 if i == idx else i
                    proof.append(level[sibling_idx])

                if left < right:
                    next_level.append(Web3.keccak(left + right))
                else:
                    next_level.append(Web3.keccak(right + left))
            else:
                next_level.append(level[i])
        level = next_level
        idx = idx // 2
    return proof

def test():
    print("Compiling contracts...")
    compiled_sol = compile_contracts()

    print("Generating SPHINCS- keys...")
    sk_seed = sphincs_c13.keygen()
    _, pk_root_32 = sphincs_c13.sign(b"dummy", sk_seed)
    print(f"Generated PK Root: {pk_root_32.hex()}")

    print("Deploying contracts...")
    mock_oracle = deploy_mock_oracle(compiled_sol, pk_root_32)
    anchor = deploy_contracts(compiled_sol, mock_oracle.address)

    print("Creating batch of evidences...")
    evidences = [f"Evidence {i}".encode() for i in range(50)]
    leaf_hashes = [Web3.keccak(e) for e in evidences]
    leaf_hashes.sort()

    root_hash = merkle_root(leaf_hashes)
    print(f"Batch Root Hash: {root_hash.hex()}")

    tick = mock_oracle.functions.tick().call()
    block_hash = w3.eth.get_block('latest')['hash']

    msg_to_sign = root_hash + tick.to_bytes(8, 'big') + block_hash
    msg_hash = Web3.keccak(msg_to_sign)

    print("Signing message with SPHINCS- (this may take a minute due to grinding)...")
    signature, pk_root_32_verified = sphincs_c13.sign(msg_hash, sk_seed)
    assert pk_root_32 == pk_root_32_verified, "PK root mismatch"
    print("Signed!")

    print("Submitting to EnterEvidenceAnchor...")
    try:
        tx_hash = anchor.functions.anchorBatch(
            root_hash,
            tick,
            block_hash,
            signature
        ).transact({'from': agent})

        tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        print(f"Transaction successful! Gas used: {tx_receipt.gasUsed}")
    except Exception as e:
        print(f"Transaction failed: {e}")
        return

    print("Verifying evidence on-chain...")
    evidence_idx = 5
    evidence_hash = leaf_hashes[evidence_idx]
    proof = merkle_proof(leaf_hashes, evidence_idx)

    is_valid = anchor.functions.verifyEvidence(evidence_hash, root_hash, proof).call()
    print(f"Evidence {evidence_idx} valid? {is_valid}")
    assert is_valid, "Evidence verification failed!"

if __name__ == '__main__':
    test()
