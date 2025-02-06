# cyberkrill

<img src="https://github.com/user-attachments/assets/246dc789-4a2d-4040-afeb-3ac9045dddfb" width="200" />

## Description

CLI utilities for Bitcoin and the Lightning Network

## Installation

Just use the nix flake:
```sh
nix run 'git+https://github.com/douglaz/cyberkrill.git' 
```

## Usage

### Examples

```sh
$ cyberkrill decode invoice lnbc99810310n1pju0sy7pp555srgtgcg6t4jr4j5v0jysgee4zy6nr4msylnycfjezxm5w6t3csdy9wdmkzupq95s8xcmjd9c8gw3qx5cnyvrrvymrwvnrxgmrzd3cxsckxdf4v3jxgcmzx9jxgenpxserjenyxv6nzwf3vsmnyctxvsuxvdehvdnrswryxgcnzdf5ve3rjvph8q6njcqzxgxq97zvuqrzjqgwf02g2gy0l9vgdc25wxt0z72wjlfyagxlmk54ag9hyvrdsw37smapyqqqqqqqq2qqqqqqqqqqqqqqq9qsp59ge5l9ndweyes4ntfrws3a3tshpkqt8eysuxnt5pmucy9hvxthmq9qyyssqaqwn0j2jf2xvcv42yl9p0yaw4t6gcqld2t44cmnfud49dxgl3dnpnjpj75kaf22yuynqtc8uzmtuckzxvfunxnr405gud8cexc5axqqphlk58z
{
  "network": "bitcoin",
  "amount_msats": 9981031000,
  "timestamp_millis": 1707589790000,
  "payment_hash": "a520342d184697590eb2a31f224119cd444d4c75dc09f9930996446dd1da5c71",
  "payment_secret": "2a334f966d764998566b48dd08f62b85c3602cf9243869ae81df3042dd865df6",
  "description": "swap - script: 5120ca672c2616841c55dddcb1ddfa429fd35191d72afd8f77cf88d21154fb907859",
  "description_hash": null,
  "destination": "03fb2a0ca79c005f493f1faa83071d3a937cf220d4051dc48b8fe3a087879cf14a",
  "expiry_seconds": 31536000,
  "min_final_cltv_expiry": 200,
  "fallback_addresses": [],
  "routes": [
    [
      {
        "src_node_id": "021c97a90a411ff2b10dc2a8e32de2f29d2fa49d41bfbb52bd416e460db0747d0d",
        "short_channel_id": 17592186044416000080,
        "fees": {
          "base_msat": 0,
          "proportional_millionths": 0
        },
        "cltv_expiry_delta": 40,
        "htlc_minimum_msat": null,
        "htlc_maximum_msat": null
      }

    ]
  ]
}
```
