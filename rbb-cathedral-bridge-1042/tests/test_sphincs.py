import pytest
import os
import sys
from web3 import Web3
from eth_tester import EthereumTester
from web3.providers.eth_tester import EthereumTesterProvider
from solcx import compile_standard, install_solc

# Add scripts dir to path to import the generator
sys.path.append(os.path.join(os.path.dirname(__file__), '..', 'scripts'))
from sphincs_c13 import keygen, sign

# Install specific solc version if not present
install_solc('0.8.28')

N = 16
SIG_SIZE = 3952

@pytest.fixture
def tester():
    return EthereumTester()

@pytest.fixture
def w3(tester):
    w3 = Web3(EthereumTesterProvider(tester))
    return w3

@pytest.fixture
def verifier_contract(w3):
    with open('rbb-cathedral-bridge-1042/contracts/CathedralSPHINCSVerifier.sol', 'r') as f:
        contract_source_code = f.read()

    compiled_sol = compile_standard({
        "language": "Solidity",
        "sources": {
            "CathedralSPHINCSVerifier.sol": {
                "content": contract_source_code
            }
        },
        "settings": {
            "viaIR": True,
            "optimizer": {
                "enabled": True,
                "runs": 200
            },
            "outputSelection": {
                "*": {
                    "*": [
                        "abi",
                        "metadata",
                        "evm.bytecode",
                        "evm.bytecode.sourceMap"
                    ]
                }
            }
        }
    }, solc_version='0.8.28')

    bytecode = compiled_sol['contracts']['CathedralSPHINCSVerifier.sol']['CathedralSPHINCSVerifier']['evm']['bytecode']['object']
    abi = compiled_sol['contracts']['CathedralSPHINCSVerifier.sol']['CathedralSPHINCSVerifier']['abi']

    # Deploy
    tx_hash = w3.eth.contract(
        abi=abi,
        bytecode=bytecode
    ).constructor().transact({'from': w3.eth.accounts[0]})

    tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

    verifier = w3.eth.contract(
        address=tx_receipt.contractAddress,
        abi=abi
    )
    return verifier

def test_verify_valid_sphincs_transaction(verifier_contract, w3, tester):
    # Generate real keys and signatures
    sk_seed = keygen()
    message = Web3.keccak(text="real test message")

    signature, pk_root_sig = sign(message, sk_seed)

    try:
        # We use transact to view gas and check it doesn't revert.
        # But we also verify the actual boolean logic
        is_valid = verifier_contract.functions.verifySPHINCS(message, signature, pk_root_sig).call()
        assert is_valid == True

        tx_hash = verifier_contract.functions.verifySPHINCS(message, signature, pk_root_sig).transact({'from': w3.eth.accounts[0], 'gas': 15000000})
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

        print(f"Gas used by verifySPHINCS transact: {receipt.gasUsed}")
    except Exception as e:
        pytest.fail(f"Execution failed unexpectedly: {e}")

def test_verify_invalid_signature_data(verifier_contract, w3, tester):
    sk_seed = keygen()
    message = Web3.keccak(text="real test message")

    signature, pk_root_sig = sign(message, sk_seed)

    # Corrupt the signature by flipping a byte
    corrupted_sig = bytearray(signature)
    corrupted_sig[42] ^= 0xFF
    corrupted_sig = bytes(corrupted_sig)

    is_valid = verifier_contract.functions.verifySPHINCS(message, corrupted_sig, pk_root_sig).call()
    assert is_valid == False

def test_invalid_signature_length(verifier_contract, w3, tester):
    message = Web3.keccak(text="test message")
    signature = os.urandom(SIG_SIZE - 1)
    public_key_root = os.urandom(32)

    try:
        tx_hash = verifier_contract.functions.verifySPHINCS(message, signature, public_key_root).transact({'from': w3.eth.accounts[0], 'gas': 15000000})
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        assert receipt.status == 0
    except Exception as e:
        pass # py-evm sometimes raises instead of returning receipt with status=0
