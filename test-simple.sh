#!/usr/bin/env bash

# Simple testnet xpub descriptor
DESCRIPTOR="wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)"

echo "Testing BDK list-utxos with testnet xpub descriptor (no blockchain scan)..."
nix develop -c cargo run -- bdk-list-utxos --descriptor "$DESCRIPTOR" --network testnet

echo -e "\n\nTesting with mainnet xpub descriptor..."
MAINNET_DESC="wpkh([f3f161df/84h/0h/0h]xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/0/*)"
nix develop -c cargo run -- bdk-list-utxos --descriptor "$MAINNET_DESC" --network mainnet