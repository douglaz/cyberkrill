#!/usr/bin/env bash

echo "=== Testing Multipath Descriptor Support ==="
echo

# Simple multipath descriptor
MULTIPATH_DESC="wpkh([c258d2e4/84h/0h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/<0;1>/*)"

echo "Testing multipath descriptor: $MULTIPATH_DESC"
echo "This will expand to:"
echo "  - External: .../0/*"
echo "  - Internal (change): .../1/*"
echo

nix develop -c cargo run -- bdk-list-utxos --descriptor "$MULTIPATH_DESC" --network testnet

echo -e "\n\nTesting complex multisig with multipath (from test.sh):"
echo "This descriptor has 8 xpubs with <0;1>/* multipath, expanding to 16 total descriptors"
DESCRIPTOR="wsh(sortedmulti(4,[f6f490c1/48h/0h/0h/2h]xpub6EdyZVuEz23YwW9mj2iuy3Q9bDyQEGMuPcikKGptXindHNmije4X4ZHkfMrtERCTFxWKmtmYRrpTroYvgqvWoGtH9tnVdNme9WvsvFRsbEB/<0;1>/*,[1f0bb8e6/48h/0h/0h/2h]xpub6Enyv7EQREnCyDHFMZn6sxX1Ta6xkp4USkZvXvq9DMzUYmW87wsKu923P6nHpQ451NADpq4QNt3gHVSwcDjg6vtYHjJ9hBo4iQxcPR8WcNi/<0;1>/*,[16c265bc/48h/0h/0h/2h]xpub6EC1YDW5CyqA9ijMraaTfRAeHWkTEW9g7yetQiZo5wPadEfcR3ShYjXirmd6grWUMXRDphxbhMYDLWqvS8CPXQ21RCqdfoFJdGbYHs4mCfg/<0;1>/*,[ca1f0f62/48h/0h/0h/2h]xpub6EUJkREtU9g7VWiNHooyTWxab6XfmVPtESNjcZdoPQjmqQTZsjW6GowBz18irMAK9KpG2f9kXufR97vXiPYT94cckQDtDQzcZ9EoUzf58U7/<0;1>/*,[830f291c/48h/0h/0h/2h]xpub6EJJxcHRzvSiCTHfe664u6Cfe1KWca3XxyKZ4CgccNX2qZBuAjBiQXjXcTsqgrt2uYq3sviWSxRUBcufBJgbEv77qZ2NAQqcVYH9erTuHnZ/<0;1>/*,[4657246a/48h/0h/0h/2h]xpub6ETjwD2efxVzxV49rsu7AoygGsFmt9EN2M6GsSmkG7YpQPJHQ4GDmVz6gj8hdm3N51g7rkGjbVUc9ijBdrBsztXKorxQUjYKM2CtKX4dSNt/<0;1>/*,[cf1972ee/48h/0h/0h/2h]xpub6F667oTNk1JykmXDaQFrUXvS9we3SuV9TnTrCEtnW1mRrU7gR2gPJ4s61GsyXtALRsF6xiyir1AVU2R1WKXXzy9d565zq6iBForibAXU1Mm/<0;1>/*,[e01c4c16/48h/0h/0h/2h]xpub6FH63JNhHQDotm8bv6mHAoon33A9pN5nR9ALWJzNmpRzZAW32FNYv9jmvFDcpQZqa4t7ERdHsz8xWPf9DabEYzHiJRe2dYziHwt1eno2w9v/<0;1>/*))"

nix develop -c cargo run -- bdk-list-utxos --descriptor "$DESCRIPTOR" --network mainnet 2>&1 | grep -E "(Warning:|total_count|utxos)"