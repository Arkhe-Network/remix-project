# EnterOS × Cathedral Integration Bridge (enter-cathedral-bridge)

The **EnterOS × Cathedral Integration Bridge** acts as an intermediary component to integrate EnterOS (an AI-based legal dispute platform) with the Cathedral ARKHE architecture (post-quantum time verification). This module enables quantum-timestamp anchoring and SPHINCS+ signature verification for legal evidence and acts.

## Contracts
- `QuantumTimestampOracle.sol`: An interface representing an oracle that provides post-quantum time ticks and signatures.
- `EnterEvidenceAnchor.sol`: Stores Merkle root hashes of batches of evidence, along with the corresponding quantum tick, signature, and blockhash. It relies on the CathedralSPHINCSVerifier for signature verification.

## Scripts
- `test_batch_anchoring.py`: Python script simulating the batch evidence anchoring process using Web3, PyEVM and `sphincs_c13.py`.
