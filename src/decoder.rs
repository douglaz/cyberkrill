use anyhow::{Context, Result};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use url::Url;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct InvoiceOutput {
    pub network: String,
    pub amount_msats: Option<u64>,
    pub timestamp_millis: u128,
    pub payment_hash: String,
    pub payment_secret: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub destination: String,
    pub expiry_seconds: u64,
    pub min_final_cltv_expiry: u64,
    pub fallback_addresses: Vec<String>,
    pub routes: Vec<Vec<RouteHintHopOutput>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RouteHintHopOutput {
    pub src_node_id: String,
    pub short_channel_id: u64,
    pub fees: RoutingFeesOutput,
    pub cltv_expiry_delta: u16,
    pub htlc_minimum_msat: Option<u64>,
    pub htlc_maximum_msat: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RoutingFeesOutput {
    pub base_msat: u32,
    pub proportional_millionths: u32,
}

impl From<&lightning_invoice::RouteHintHop> for RouteHintHopOutput {
    fn from(hop: &lightning_invoice::RouteHintHop) -> Self {
        Self {
            src_node_id: hop.src_node_id.to_string(),
            short_channel_id: hop.short_channel_id,
            fees: (&hop.fees).into(),
            cltv_expiry_delta: hop.cltv_expiry_delta,
            htlc_minimum_msat: hop.htlc_minimum_msat,
            htlc_maximum_msat: hop.htlc_maximum_msat,
        }
    }
}

impl From<&lightning_invoice::RoutingFees> for RoutingFeesOutput {
    fn from(fees: &lightning_invoice::RoutingFees) -> Self {
        Self {
            base_msat: fees.base_msat,
            proportional_millionths: fees.proportional_millionths,
        }
    }
}

impl From<lightning_invoice::Bolt11Invoice> for InvoiceOutput {
    fn from(invoice: lightning_invoice::Bolt11Invoice) -> Self {
        Self {
            network: invoice.network().to_string(),
            amount_msats: invoice.amount_milli_satoshis(),
            timestamp_millis: invoice.duration_since_epoch().as_millis(),
            payment_hash: invoice.payment_hash().to_string(),
            payment_secret: hex::encode(invoice.payment_secret().0),
            description: match invoice.description() {
                lightning_invoice::Bolt11InvoiceDescriptionRef::Direct(description) => {
                    Some(description.to_string())
                }
                lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(_sha256) => None,
            },
            description_hash: match invoice.description() {
                lightning_invoice::Bolt11InvoiceDescriptionRef::Direct(_description) => None,
                lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(sha256) => {
                    Some(sha256.0.to_string())
                }
            },
            destination: invoice
                .payee_pub_key()
                .map(ToString::to_string)
                .unwrap_or_else(|| invoice.recover_payee_pub_key().to_string()),
            expiry_seconds: invoice.expiry_time().as_secs(),
            min_final_cltv_expiry: invoice.min_final_cltv_expiry_delta(),
            fallback_addresses: invoice
                .fallback_addresses()
                .iter()
                .map(|a| a.to_string())
                .collect(),
            routes: invoice
                .route_hints()
                .iter()
                .map(|hints| hints.0.iter().map(|hop| hop.into()).collect())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LnurlOutput {
    pub bech32: String,
    pub url: String,
    pub host: String,
    pub base: String,
    pub query: Option<String>,
    pub query_params: std::collections::HashMap<String, String>,
}

pub fn decode_invoice(input: &str) -> Result<InvoiceOutput> {
    let invoice = Bolt11Invoice::from_str(input)
        .map_err(|e| anyhow::anyhow!("Failed to parse invoice: {e:?}"))?;
    Ok(InvoiceOutput::from(invoice))
}

pub fn decode_lnurl(input: &str) -> Result<LnurlOutput> {
    let input = input.trim();
    anyhow::ensure!(
        input.to_uppercase().starts_with("LNURL"),
        "Input must start with 'LNURL'"
    );

    let (hrp, data, _variant) = bech32::decode(input)?;
    anyhow::ensure!(
        hrp == "lnurl",
        "Invalid HRP (human-readable part): expected 'lnurl', got '{hrp}'"
    );

    // convert Vec<u5> to Vec<u8>
    let decoded_bytes = bech32::convert_bits(&data, 5, 8, false)?;
    let url_str = String::from_utf8(decoded_bytes)?;

    let url = Url::parse(&url_str)?;
    let mut query_params = HashMap::new();
    for (key, value) in url.query_pairs() {
        query_params.insert(key.to_string(), value.to_string());
    }

    let host = url
        .host_str()
        .with_context(|| format!("Failed to get host from URL: {url_str}"))?;
    let base = format!("{scheme}://{host}/", scheme = url.scheme());

    Ok(LnurlOutput {
        bech32: input.to_string(),
        url: url_str,
        host: host.to_owned(),
        base,
        query: url.query().map(|q| q.to_string()),
        query_params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_invoice() -> anyhow::Result<()> {
        let invoice = "lnbc99810310n1pju0sy7pp555srgtgcg6t4jr4j5v0jysgee4zy6nr4msylnycfjezxm5w6t3csdy9wdmkzupq95s8xcmjd9c8gw3qx5cnyvrrvymrwvnrxgmrzd3cxsckxdf4v3jxgcmzx9jxgenpxserjenyxv6nzwf3vsmnyctxvsuxvdehvdnrswryxgcnzdf5ve3rjvph8q6njcqzxgxq97zvuqrzjqgwf02g2gy0l9vgdc25wxt0z72wjlfyagxlmk54ag9hyvrdsw37smapyqqqqqqqq2qqqqqqqqqqqqqqq9qsp59ge5l9ndweyes4ntfrws3a3tshpkqt8eysuxnt5pmucy9hvxthmq9qyyssqaqwn0j2jf2xvcv42yl9p0yaw4t6gcqld2t44cmnfud49dxgl3dnpnjpj75kaf22yuynqtc8uzmtuckzxvfunxnr405gud8cexc5axqqphlk58z";
        let output = decode_invoice(invoice)?;

        // Basic invoice properties
        assert_eq!(output.network, "bitcoin");
        assert_eq!(output.amount_msats, Some(9981031000));
        assert_eq!(output.timestamp_millis, 1707589790000);

        // Payment details
        assert_eq!(
            output.payment_hash,
            "a520342d184697590eb2a31f224119cd444d4c75dc09f9930996446dd1da5c71"
        );
        assert_eq!(
            output.payment_secret,
            "2a334f966d764998566b48dd08f62b85c3602cf9243869ae81df3042dd865df6"
        );
        assert_eq!(
            output.destination,
            "03fb2a0ca79c005f493f1faa83071d3a937cf220d4051dc48b8fe3a087879cf14a"
        );

        // Description
        assert_eq!(output.description, Some("swap - script: 5120ca672c2616841c55dddcb1ddfa429fd35191d72afd8f77cf88d21154fb907859".to_string()));
        assert_eq!(output.description_hash, None);

        // Expiry and CLTV
        assert_eq!(output.expiry_seconds, 31536000);
        assert_eq!(output.min_final_cltv_expiry, 200);

        // Routes
        assert_eq!(output.routes.len(), 1);
        let route = &output.routes[0][0];
        assert_eq!(
            route.src_node_id,
            "021c97a90a411ff2b10dc2a8e32de2f29d2fa49d41bfbb52bd416e460db0747d0d"
        );
        assert_eq!(route.short_channel_id, 17592186044416000080);
        assert_eq!(route.cltv_expiry_delta, 40);
        assert_eq!(route.fees.base_msat, 0);
        assert_eq!(route.fees.proportional_millionths, 0);

        // Empty fields
        assert!(output.fallback_addresses.is_empty());

        Ok(())
    }

    #[test]
    fn test_decode_lnurl() -> anyhow::Result<()> {
        let lnurl = "LNURL1DP68GURN8GHJ7UM9WFMXJCM99E5K7TELWY7NXENRXVMRGDTZXSENJCM98PJNWXQ96S9";
        let output = decode_lnurl(lnurl)?;

        assert_eq!(output.bech32, lnurl);
        assert_eq!(output.url, "https://service.io/?q=3fc3645b439ce8e7");
        assert_eq!(output.host, "service.io");
        assert_eq!(output.base, "https://service.io/");
        assert_eq!(output.query, Some("q=3fc3645b439ce8e7".to_string()));
        assert_eq!(output.query_params.len(), 1);
        assert_eq!(
            output.query_params.get("q"),
            Some(&"3fc3645b439ce8e7".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_decode_invalid_lnurl() {
        let invalid_lnurl = "not_an_lnurl";
        let result = decode_lnurl(invalid_lnurl);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_invalid_invoice() {
        let invalid_invoice = "not_an_invoice";
        let result = decode_invoice(invalid_invoice);
        assert!(result.is_err());
    }
}
