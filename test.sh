#!/usr/bin/env bash

# Test script for cyberkrill with unified BDK backend support

# Example multisig descriptor
DESCRIPTOR="wsh(sortedmulti(4,[f6f490c1/48h/0h/0h/2h]xpub6EdyZVuEz23YwW9mj2iuy3Q9bDyQEGMuPcikKGptXindHNmije4X4ZHkfMrtERCTFxWKmtmYRrpTroYvgqvWoGtH9tnVdNme9WvsvFRsbEB/<0;1>/*,[1f0bb8e6/48h/0h/0h/2h]xpub6Enyv7EQREnCyDHFMZn6sxX1Ta6xkp4USkZvXvq9DMzUYmW87wsKu923P6nHpQ451NADpq4QNt3gHVSwcDjg6vtYHjJ9hBo4iQxcPR8WcNi/<0;1>/*,[16c265bc/48h/0h/0h/2h]xpub6EC1YDW5CyqA9ijMraaTfRAeHWkTEW9g7yetQiZo5wPadEfcR3ShYjXirmd6grWUMXRDphxbhMYDLWqvS8CPXQ21RCqdfoFJdGbYHs4mCfg/<0;1>/*,[ca1f0f62/48h/0h/0h/2h]xpub6EUJkREtU9g7VWiNHooyTWxab6XfmVPtESNjcZdoPQjmqQTZsjW6GowBz18irMAK9KpG2f9kXufR97vXiPYT94cckQDtDQzcZ9EoUzf58U7/<0;1>/*,[830f291c/48h/0h/0h/2h]xpub6EJJxcHRzvSiCTHfe664u6Cfe1KWca3XxyKZ4CgccNX2qZBuAjBiQXjXcTsqgrt2uYq3sviWSxRUBcufBJgbEv77qZ2NAQqcVYH9erTuHnZ/<0;1>/*,[4657246a/48h/0h/0h/2h]xpub6ETjwD2efxVzxV49rsu7AoygGsFmt9EN2M6GsSmkG7YpQPJHQ4GDmVz6gj8hdm3N51g7rkGjbVUc9ijBdrBsztXKorxQUjYKM2CtKX4dSNt/<0;1>/*,[cf1972ee/48h/0h/0h/2h]xpub6F667oTNk1JykmXDaQFrUXvS9we3SuV9TnTrCEtnW1mRrU7gR2gPJ4s61GsyXtALRsF6xiyir1AVU2R1WKXXzy9d565zq6iBForibAXU1Mm/<0;1>/*,[e01c4c16/48h/0h/0h/2h]xpub6FH63JNhHQDotm8bv6mHAoon33A9pN5nR9ALWJzNmpRzZAW32FNYv9jmvFDcpQZqa4t7ERdHsz8xWPf9DabEYzHiJRe2dYziHwt1eno2w9v/<0;1>/*))"

# Simple single-sig descriptor for testing
SIMPLE_DESCRIPTOR="wpkh([fingerprint/84'/0'/0']xpub...)"

echo "=== CyberKrill Backend Test Suite ==="
echo "Testing unified BDK backend support"
echo

# Test 1: List UTXOs with different backends
echo "1. Testing list-utxos command with different backends:"
echo

echo "   a) Bitcoin Core backend (default):"
#nix develop -c cargo run -- list-utxos --descriptor "$DESCRIPTOR" --bitcoin-dir ~/libre

echo "   b) Electrum backend:"
#nix develop -c cargo run -- list-utxos --descriptor "$DESCRIPTOR" --electrum ssl://electrum.blockstream.info:50002

echo "   c) Esplora backend:"
nix develop -c cargo run -- list-utxos --descriptor "$DESCRIPTOR" --esplora https://blockstream.info/api

# Test 2: Create PSBT with different backends
echo
echo "2. Testing create-psbt command with different backends:"
echo

# Note: Replace with actual UTXO txid:vout and destination address
TXID="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
VOUT="0"
DESTINATION="bc1qexample..."

echo "   a) Bitcoin Core backend:"
#nix develop -c cargo run -- create-psbt --descriptor "$DESCRIPTOR" --bitcoin-dir ~/libre \
#  --inputs "${TXID}:${VOUT}" \
#  --outputs "${DESTINATION}:0.001" \
#  --fee-rate 10sats

echo "   b) Electrum backend:"
#nix develop -c cargo run -- create-psbt --descriptor "$DESCRIPTOR" --electrum ssl://electrum.blockstream.info:50002 \
#  --inputs "${TXID}:${VOUT}" \
#  --outputs "${DESTINATION}:0.001" \
#  --fee-rate 10sats

echo "   c) Esplora backend:"
#nix develop -c cargo run -- create-psbt --descriptor "$DESCRIPTOR" --esplora https://blockstream.info/api \
#  --inputs "${TXID}:${VOUT}" \
#  --outputs "${DESTINATION}:0.001" \
#  --fee-rate 10sats

# Test 3: Create funded PSBT (automatic input selection)
echo
echo "3. Testing create-funded-psbt command:"
echo

echo "   a) With Electrum backend:"
#nix develop -c cargo run -- create-funded-psbt --descriptor "$DESCRIPTOR" --electrum ssl://electrum.blockstream.info:50002 \
#  --outputs "${DESTINATION}:0.001" \
#  --fee-rate 5sats

# Test 4: Move UTXOs (consolidation)
echo
echo "4. Testing move-utxos command:"
echo

echo "   a) With Esplora backend:"
#nix develop -c cargo run -- move-utxos --descriptor "$DESCRIPTOR" --esplora https://blockstream.info/api \
#  --inputs "${TXID}:${VOUT}" \
#  --destination "${DESTINATION}" \
#  --fee-rate 10sats

echo
echo "Test suite complete. Uncomment the commands you want to run."
