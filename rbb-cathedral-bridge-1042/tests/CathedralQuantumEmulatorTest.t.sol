// SPDX-License-Identifier: Apache-2.0
// Testes de integração do emulador quântico na RBB Chain testnet
// forge test --match-contract CathedralQuantumEmulatorTest --gas-report -vvv

pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../contracts/CathedralSPHINCSVerifier.sol";
import "../src/QuantumTimestampOracle.sol";

contract CathedralQuantumEmulatorTest is Test {
    CathedralSPHINCSVerifier verifier;
    QuantumTimestampOracle oracle;

    // Endereço do emulador (simulado)
    address emulatorAddress = address(0x1234);
    bytes32 publicKeyRoot = bytes32(hex"5706b9367444e0c58009af290b907647");

    function setUp() public {
        verifier = new CathedralSPHINCSVerifier();
        oracle = new QuantumTimestampOracle(emulatorAddress, address(verifier));
    }

    // ============================================================
    // TESTE 1: Tick válido (caminho feliz)
    // ============================================================
    function testValidTick() public {
        // Simula tick do emulador
        uint64 tickId = 100;
        bytes32 blockHash = blockhash(block.number - 1);
        bytes memory message = abi.encodePacked(tickId, blockHash);

        // Assinatura do emulador (stub)
        bytes memory signature = _signTick(message);

        // Verifica no contrato
        bool valid = oracle.verifyTick(tickId, blockHash, signature, publicKeyRoot);
        assertTrue(valid, "Tick válido deve ser aceito");
    }

    // ============================================================
    // TESTE 2: Ataque de avanço rápido
    // ============================================================
    function testAttackFastForward() public {
        uint64 currentTick = oracle.latestTick();
        uint64 futureTick = currentTick + 1000;  // Avanço de 1000 ticks

        bytes32 blockHash = blockhash(block.number - 1);
        bytes memory message = abi.encodePacked(futureTick, blockHash);
        bytes memory signature = _signTick(message);

        // Deve reverter: tick avança mais que janela máxima (5)
        vm.expectRevert("Tick too far in future");
        oracle.verifyTick(futureTick, blockHash, signature, publicKeyRoot);
    }

    // ============================================================
    // TESTE 3: Ataque de atraso
    // ============================================================
    function testAttackDelay() public {
        // Simula atraso: tick antigo
        uint64 oldTick = oracle.latestTick() - 100;

        bytes32 blockHash = blockhash(block.number - 1);
        bytes memory message = abi.encodePacked(oldTick, blockHash);
        bytes memory signature = _signTick(message);

        // Deve reverter: tick já passou
        vm.expectRevert("Tick already passed");
        oracle.verifyTick(oldTick, blockHash, signature, publicKeyRoot);
    }

    // ============================================================
    // TESTE 4: Ataque de repetição
    // ============================================================
    function testAttackReplay() public {
        uint64 tickId = 100;
        bytes32 oldBlockHash = blockhash(block.number - 2);
        bytes32 newBlockHash = blockhash(block.number - 1);

        // Assinatura do tick com hash antigo
        bytes memory message = abi.encodePacked(tickId, oldBlockHash);
        bytes memory signature = _signTick(message);

        // Tenta reapresentar com hash novo
        // Deve falhar: assinatura não corresponde à mensagem
        bool valid = oracle.verifyTick(tickId, newBlockHash, signature, publicKeyRoot);
        assertFalse(valid, "Replay deve ser detectado");
    }

    // ============================================================
    // TESTE 5: Ataque de deriva de frequência
    // ============================================================
    function testAttackFrequencyDrift() public {
        // Simula deriva: múltiplos ticks em sequência rápida
        for (uint64 i = 0; i < 10; i++) {
            uint64 tickId = oracle.latestTick() + 1;
            bytes32 blockHash = blockhash(block.number - 1);
            bytes memory message = abi.encodePacked(tickId, blockHash);
            bytes memory signature = _signTick(message);

            // Avança o tempo artificialmente (simula deriva)
            vm.warp(block.timestamp + 1);

            bool valid = oracle.verifyTick(tickId, blockHash, signature, publicKeyRoot);
            // Após detectar deriva, o oráculo deve rejeitar
            if (i > 5) {
                assertFalse(valid, "Deriva deve ser detectada");
            }
        }
    }

    // ============================================================
    // TESTE 6: Ataque de 51% combinado
    // ============================================================
    function testAttack51Percent() public {
        // Simula múltiplos emuladores maliciosos
        address[] memory maliciousOracles = new address[](3);
        maliciousOracles[0] = address(0xBAD1);
        maliciousOracles[1] = address(0xBAD2);
        maliciousOracles[2] = address(0xBAD3);

        // Tenta submeter ticks falsos de múltiplas fontes
        for (uint i = 0; i < maliciousOracles.length; i++) {
            vm.prank(maliciousOracles[i]);

            uint64 tickId = 999;
            bytes32 blockHash = blockhash(block.number - 1);
            bytes memory message = abi.encodePacked(tickId, blockHash);
            bytes memory signature = _signTick(message);

            // Apenas o oráculo autorizado pode submeter
            if (maliciousOracles[i] != emulatorAddress) {
                vm.expectRevert("Unauthorized oracle");
            }
            oracle.verifyTick(tickId, blockHash, signature, publicKeyRoot);
        }
    }

    // ============================================================
    // TESTE 7: Gas report
    // ============================================================
    function testGasReport() public {
        uint64 tickId = 100;
        bytes32 blockHash = blockhash(block.number - 1);
        bytes memory message = abi.encodePacked(tickId, blockHash);
        bytes memory signature = _signTick(message);

        uint256 gasBefore = gasleft();
        oracle.verifyTick(tickId, blockHash, signature, publicKeyRoot);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for tick verification:", gasUsed);
        assertLt(gasUsed, 150000, "Gas should be under 150k");
    }

    // ============================================================
    // HELPER: Assinatura stub
    // ============================================================
    function _signTick(bytes memory message) internal pure returns (bytes memory) {
        // Em produção: assinatura SPHINCS- real do emulador
        // Aqui: stub HMAC-SHA3-256
        return new bytes(3952);
    }
}
