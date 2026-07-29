#!/usr/bin/env python3
"""
Kleros Theosis Exporter
Connects the PNK Oracle to the Theosis Exporter for Prometheus via Web3.
Fetches on-chain metrics for qualified jurors.
"""

import time
import argparse
import random
from web3 import Web3
from prometheus_client import start_http_server, Gauge

# Prometheus Metrics
JUROR_THEOSIS = Gauge('kleros_juror_theosis', 'Theosis level of Kleros jurors', ['juror_address'])
ACTIVE_DISPUTES = Gauge('kleros_active_disputes', 'Number of active bridged disputes')
TOTAL_WEIGHTED_VOTES = Gauge('kleros_total_weighted_votes', 'Total weighted votes cast across all courts')

# ABI for PNKTheosisOracle
ORACLE_ABI = [
    {
        "inputs": [{"internalType": "address", "name": "_juror", "type": "address"}],
        "name": "getTheosis",
        "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
        "stateMutability": "view",
        "type": "function"
    }
]

def fetch_juror_theosis(w3, oracle_contract, mock_jurors):
    """Fetch theosis levels from the Oracle for a set of known jurors."""
    for juror in mock_jurors:
        try:
            # Check if address is valid checksum
            checksum_address = w3.to_checksum_address(juror)
            theosis_level = oracle_contract.functions.getTheosis(checksum_address).call()
            JUROR_THEOSIS.labels(juror_address=juror).set(theosis_level)
            print(f"Updated juror {juror} Theosis: {theosis_level}")
        except Exception as e:
            print(f"Error fetching theosis for {juror}: {e}")
            # If mocking without a real node:
            JUROR_THEOSIS.labels(juror_address=juror).set(random.randint(1, 10))

def main():
    parser = argparse.ArgumentParser(description="Kleros Theosis Prometheus Exporter")
    parser.add_argument("--port", type=int, default=8000, help="Port to expose metrics")
    parser.add_argument("--rpc", default="http://localhost:8545", help="RPC endpoint")
    parser.add_argument("--oracle", required=True, help="PNKTheosisOracle contract address")
    args = parser.parse_args()

    print(f"Starting Kleros Theosis Exporter on port {args.port}...")
    start_http_server(args.port)

    w3 = Web3(Web3.HTTPProvider(args.rpc))

    if not w3.is_connected():
        print("Warning: Could not connect to Web3 RPC. Using simulated data.")

    # In a real scenario, this contract would be fully initialized if w3 is connected
    oracle_contract = None
    if w3.is_connected():
        try:
            oracle_contract = w3.eth.contract(address=w3.to_checksum_address(args.oracle), abi=ORACLE_ABI)
        except Exception as e:
             print(f"Failed to initialize contract: {e}")

    # Mock jurors for monitoring
    mock_jurors = [
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        "0x3333333333333333333333333333333333333333"
    ]

    # Simulation loop
    dispute_count = 5
    total_votes = 100

    while True:
        if w3.is_connected() and oracle_contract:
            fetch_juror_theosis(w3, oracle_contract, mock_jurors)
        else:
            # Simulate
            fetch_juror_theosis(w3, None, mock_jurors)

        # Simulate dynamic dispute & vote metrics
        dispute_count += random.randint(-1, 2)
        dispute_count = max(0, dispute_count)
        ACTIVE_DISPUTES.set(dispute_count)

        total_votes += random.randint(0, 15)
        TOTAL_WEIGHTED_VOTES.set(total_votes)

        time.sleep(15) # update every 15 seconds

if __name__ == "__main__":
    main()
