#!/usr/bin/env bash

echo "=== BDK Wallet List UTXOs Test ==="
echo

# Test 1: Local wallet (no blockchain scan)
echo "1. Testing local wallet with mainnet descriptor (no blockchain scan):"
MAINNET_DESC="wpkh([f3f161df/84h/0h/0h]xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/0/*)"
nix develop -c cargo run -- bdk-list-utxos --descriptor "$MAINNET_DESC" --network mainnet

echo -e "\n2. Testing with non-SSL Electrum server (if you have one running locally):"
echo "Skipping Electrum test for now (requires SSL setup or local Electrum server)"
# To test with Electrum, you could run:
# nix develop -c cargo run -- bdk-list-utxos --descriptor "YOUR_DESCRIPTOR" --network testnet --electrum "tcp://localhost:50001" --stop-gap 20

echo -e "\n3. Testing help output:"
nix develop -c cargo run -- bdk-list-utxos --help