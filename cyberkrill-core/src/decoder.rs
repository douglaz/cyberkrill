use anyhow::{Context, Result, bail};
use bitcoin::hashes::Hash;
use chrono::{DateTime, Utc};
use lightning_invoice::{Bolt11Invoice, Currency, InvoiceBuilder};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, fmt, str::FromStr, time::Duration};
use strum::{Display, EnumString};
use url::Url;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[strum(serialize = "bitcoin", serialize = "mainnet")]
    #[serde(rename = "bitcoin", alias = "mainnet")]
    Bitcoin,
    Testnet,
    Regtest,
    Signet,
    Simnet,
}

impl Network {
    /// Convert to lightning-invoice Currency
    pub fn to_currency(&self) -> Currency {
        match self {
            Network::Bitcoin => Currency::Bitcoin,
            Network::Testnet => Currency::BitcoinTestnet,
            Network::Regtest => Currency::Regtest,
            Network::Signet => Currency::Signet,
            Network::Simnet => Currency::Simnet,
        }
    }

    /// Convert from lightning-invoice Currency
    pub fn from_currency(currency: &Currency) -> Self {
        match currency {
            Currency::Bitcoin => Network::Bitcoin,
            Currency::BitcoinTestnet => Network::Testnet,
            Currency::Regtest => Network::Regtest,
            Currency::Signet => Network::Signet,
            Currency::Simnet => Network::Simnet,
        }
    }
}

/// Strongly typed wrapper for payment hash (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaymentHash([u8; 32]);

impl PaymentHash {
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            bail!("Payment hash must be exactly 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(slice);
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl Serialize for PaymentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PaymentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        Self::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Strongly typed wrapper for payment secret (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentSecret([u8; 32]);

impl PaymentSecret {
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            bail!("Payment secret must be exactly 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(slice);
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl Serialize for PaymentSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PaymentSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        Self::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Strongly typed wrapper for public key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(bitcoin::secp256k1::PublicKey);

impl PublicKey {
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        let key =
            bitcoin::secp256k1::PublicKey::from_slice(slice).context("Invalid public key bytes")?;
        Ok(Self(key))
    }

    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str).context("Invalid hex string")?;
        Self::from_slice(&bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0.serialize())
    }

    pub fn inner(&self) -> &bitcoin::secp256k1::PublicKey {
        &self.0
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// SHA256 hash wrapper for description hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Hash([u8; 32]);

impl Sha256Hash {
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            bail!("SHA256 hash must be exactly 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(slice);
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl Serialize for Sha256Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Sha256Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        Self::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Feature status in Lightning invoice
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureStatus {
    Required,
    Optional,
}

/// Lightning invoice features
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    BasicMpp(FeatureStatus),
    PaymentMetadata(FeatureStatus),
    PaymentSecret(FeatureStatus),
    TrampolineRouting(FeatureStatus),
    UnknownBits(FeatureStatus),
    VariableLengthOnion(FeatureStatus),
    AnyOptionalBits,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct InvoiceOutput {
    pub network: Network,
    pub amount_msats: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub timestamp_millis: u128,
    pub payment_hash: PaymentHash,
    pub payment_secret: PaymentSecret,
    pub features: Vec<Feature>,
    pub description: Option<String>,
    pub description_hash: Option<Sha256Hash>,
    pub destination: PublicKey,
    pub expiry_seconds: u64,
    pub min_final_cltv_expiry: u64,
    pub fallback_addresses: Vec<String>,
    pub routes: Vec<Vec<RouteHintHopOutput>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RouteHintHopOutput {
    pub src_node_id: PublicKey,
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
        // Convert lightning-invoice PublicKey to our PublicKey wrapper
        let src_node_id = PublicKey::from_slice(&hop.src_node_id.serialize()).unwrap();
        Self {
            src_node_id,
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
        let mut features = vec![];
        if let Some(f) = invoice.features() {
            if f.requires_basic_mpp() {
                features.push(Feature::BasicMpp(FeatureStatus::Required));
            } else if f.supports_basic_mpp() {
                features.push(Feature::BasicMpp(FeatureStatus::Optional));
            }
            if f.requires_payment_metadata() {
                features.push(Feature::PaymentMetadata(FeatureStatus::Required));
            } else if f.supports_payment_metadata() {
                features.push(Feature::PaymentMetadata(FeatureStatus::Optional));
            }
            if f.requires_payment_secret() {
                features.push(Feature::PaymentSecret(FeatureStatus::Required));
            } else if f.supports_payment_secret() {
                features.push(Feature::PaymentSecret(FeatureStatus::Optional));
            }
            if f.requires_trampoline_routing() {
                features.push(Feature::TrampolineRouting(FeatureStatus::Required));
            } else if f.supports_trampoline_routing() {
                features.push(Feature::TrampolineRouting(FeatureStatus::Optional));
            }
            if f.requires_unknown_bits() {
                features.push(Feature::UnknownBits(FeatureStatus::Required));
            } else if f.supports_unknown_bits() {
                features.push(Feature::UnknownBits(FeatureStatus::Optional));
            }
            if f.requires_variable_length_onion() {
                features.push(Feature::VariableLengthOnion(FeatureStatus::Required));
            } else if f.supports_variable_length_onion() {
                features.push(Feature::VariableLengthOnion(FeatureStatus::Optional));
            }
            if f.supports_any_optional_bits() {
                features.push(Feature::AnyOptionalBits);
            }
        }

        // Convert timestamp to human-readable format
        let timestamp_millis = invoice.duration_since_epoch().as_millis();
        let timestamp_secs = (timestamp_millis / 1000) as i64;
        let timestamp_nanos = ((timestamp_millis % 1000) * 1_000_000) as u32;
        let datetime = DateTime::<Utc>::from_timestamp(timestamp_secs, timestamp_nanos)
            .unwrap_or_else(Utc::now);

        // Convert payment hash
        let payment_hash = {
            let hash_bytes = invoice.payment_hash().as_byte_array();
            PaymentHash::from_slice(hash_bytes).unwrap()
        };

        // Convert payment secret
        let payment_secret = PaymentSecret::from_slice(&invoice.payment_secret().0).unwrap();

        // Convert destination public key
        let destination = {
            let pubkey = if let Some(pk) = invoice.payee_pub_key() {
                pk.clone()
            } else {
                invoice.recover_payee_pub_key()
            };
            PublicKey::from_slice(&pubkey.serialize()).unwrap()
        };

        // Convert description hash if present
        let description_hash = match invoice.description() {
            lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(sha256) => {
                Some(Sha256Hash::from_slice(sha256.0.as_byte_array()).unwrap())
            }
            _ => None,
        };

        Self {
            network: Network::from_currency(&invoice.currency()),
            amount_msats: invoice.amount_milli_satoshis(),
            timestamp: datetime,
            timestamp_millis,
            payment_hash,
            payment_secret,
            features,
            description: match invoice.description() {
                lightning_invoice::Bolt11InvoiceDescriptionRef::Direct(description) => {
                    Some(description.to_string())
                }
                lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(_) => None,
            },
            description_hash,
            destination,
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

    let (hrp, data) = bech32::decode(input)?;
    anyhow::ensure!(
        hrp == bech32::primitives::hrp::Hrp::parse("lnurl")?,
        "Invalid HRP (human-readable part): expected 'lnurl', got '{}'",
        hrp.as_str()
    );

    let url_str = String::from_utf8(data)?;

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

// LNURL-pay structures
#[derive(Debug, Serialize, Deserialize)]
pub struct LnurlPayRequest {
    pub callback: String,
    #[serde(rename = "maxSendable")]
    pub max_sendable: u64,
    #[serde(rename = "minSendable")]
    pub min_sendable: u64,
    pub metadata: String,
    pub tag: String,
    #[serde(rename = "commentAllowed")]
    pub comment_allowed: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LnurlPayCallback {
    #[serde(rename = "pr")]
    pub payment_request: String,
    pub routes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedInvoiceOutput {
    pub lightning_address: String,
    pub amount_msats: u64,
    pub comment: Option<String>,
    pub invoice: String,
    pub decoded_invoice: InvoiceOutput,
}

/// Generate a Lightning invoice from a Lightning address using the LNURL-pay protocol.
///
/// This function implements the LNURL-pay protocol to generate Lightning invoices
/// from Lightning addresses (e.g., user@domain.com). It:
/// 1. Resolves the Lightning address to an LNURL-pay endpoint
/// 2. Fetches payment request metadata from the endpoint
/// 3. Generates an invoice with the specified amount and optional comment
///
/// # Arguments
/// * `address` - Lightning address in format user@domain.com
/// * `amount` - Amount to request (supports various formats: BTC, sats, msats)
/// * `comment` - Optional comment to include with payment request
pub async fn generate_invoice_from_address(
    address: &str,
    amount: &crate::bitcoin_rpc::AmountInput,
    comment: Option<&str>,
) -> Result<GeneratedInvoiceOutput> {
    let amount_msats = amount.as_millisats();
    // Parse lightning address
    let parts: Vec<&str> = address.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        anyhow::bail!("Invalid Lightning address format. Expected: user@domain.com");
    }

    let (user, domain) = (parts[0], parts[1]);

    // Construct the .well-known URL
    let well_known_url = format!("https://{domain}/.well-known/lnurlp/{user}");

    // Make initial request to get LNURL-pay request
    let client = reqwest::Client::new();
    let lnurl_pay_request: LnurlPayRequest =
        client.get(&well_known_url).send().await?.json().await?;

    // Validate amount
    if amount_msats < lnurl_pay_request.min_sendable
        || amount_msats > lnurl_pay_request.max_sendable
    {
        anyhow::bail!(
            "Amount {amount_msats} msats is outside allowed range: {min_sendable} - {max_sendable} msats",
            min_sendable = lnurl_pay_request.min_sendable,
            max_sendable = lnurl_pay_request.max_sendable
        );
    }

    // Validate comment length if provided
    if let Some(comment) = comment
        && let Some(max_comment_len) = lnurl_pay_request.comment_allowed
        && comment.len() > max_comment_len as usize
    {
        anyhow::bail!(
            "Comment length {comment_len} exceeds maximum allowed length: {max_comment_len}",
            comment_len = comment.len()
        );
    }

    // Build callback URL with parameters
    let mut callback_url = Url::parse(&lnurl_pay_request.callback)?;
    callback_url
        .query_pairs_mut()
        .append_pair("amount", &amount_msats.to_string());

    if let Some(comment) = comment {
        callback_url
            .query_pairs_mut()
            .append_pair("comment", comment);
    }

    // Make callback request to get invoice
    let callback_response: LnurlPayCallback = client
        .get(callback_url.as_str())
        .send()
        .await?
        .json()
        .await?;

    // Decode the received invoice
    let decoded_invoice = decode_invoice(&callback_response.payment_request)?;

    Ok(GeneratedInvoiceOutput {
        lightning_address: address.to_string(),
        amount_msats,
        comment: comment.map(|s| s.to_string()),
        invoice: callback_response.payment_request,
        decoded_invoice,
    })
}

/// Encode a Lightning invoice from JSON output structure back to BOLT11 string.
///
/// This function takes an InvoiceOutput structure (typically from decoding)
/// and reconstructs a valid BOLT11 invoice string. The invoice must be signed
/// with a private key to be valid.
///
/// # Arguments
/// * `invoice_data` - The invoice data structure to encode
/// * `private_key_hex` - The private key in hex format for signing the invoice
///
/// # Returns
/// * A BOLT11 invoice string
pub fn encode_invoice(invoice_data: &InvoiceOutput, private_key_hex: &str) -> Result<String> {
    use bitcoin::hashes::Hash as BitcoinHash;
    use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};

    // Parse the private key
    let private_key_bytes = hex::decode(private_key_hex).context("Invalid private key hex")?;
    let private_key =
        SecretKey::from_slice(&private_key_bytes).context("Invalid private key format")?;

    // Determine network/currency
    let currency = invoice_data.network.to_currency();

    // Convert payment hash to bitcoin hash
    let payment_hash =
        bitcoin::hashes::sha256::Hash::from_slice(invoice_data.payment_hash.as_bytes())
            .context("Invalid payment hash")?;

    // Convert payment secret to lightning-invoice type
    let payment_secret = lightning_invoice::PaymentSecret(*invoice_data.payment_secret.as_bytes());

    // Set timestamp
    let timestamp_secs = (invoice_data.timestamp_millis / 1000) as u64;
    let duration = Duration::from_secs(timestamp_secs);

    // Build the invoice with required fields - set description first
    let builder = if let Some(ref description) = invoice_data.description {
        InvoiceBuilder::new(currency.clone()).description(description.clone())
    } else if let Some(ref description_hash) = invoice_data.description_hash {
        let sha256 = bitcoin::hashes::sha256::Hash::from_slice(description_hash.as_bytes())
            .context("Invalid description hash")?;
        InvoiceBuilder::new(currency.clone()).description_hash(sha256)
    } else {
        // Lightning invoices require either description or description_hash
        InvoiceBuilder::new(currency.clone()).description("".to_string())
    };

    let mut builder = builder
        .payment_hash(payment_hash)
        .payment_secret(payment_secret)
        .duration_since_epoch(duration)
        .min_final_cltv_expiry_delta(invoice_data.min_final_cltv_expiry);

    // Set amount if present
    if let Some(amount_msats) = invoice_data.amount_msats {
        builder = builder.amount_milli_satoshis(amount_msats);
    }

    // Set expiry time
    builder = builder.expiry_time(Duration::from_secs(invoice_data.expiry_seconds));

    // Set payee public key
    let destination_pubkey = invoice_data.destination.inner().clone();
    builder = builder.payee_pub_key(destination_pubkey);

    // Add fallback addresses
    for fallback_str in &invoice_data.fallback_addresses {
        use bitcoin::address::NetworkUnchecked;
        use bitcoin::{Address, PubkeyHash, ScriptHash};
        use lightning_invoice::Fallback;

        // Parse address without requiring network
        let address: Address<NetworkUnchecked> =
            fallback_str.parse().context("Invalid fallback address")?;

        // Convert bitcoin::Address to lightning_invoice::Fallback
        // This is network-specific conversion
        let network = match currency {
            Currency::Bitcoin => bitcoin::Network::Bitcoin,
            Currency::BitcoinTestnet => bitcoin::Network::Testnet,
            Currency::Regtest => bitcoin::Network::Regtest,
            Currency::Signet => bitcoin::Network::Signet,
            Currency::Simnet => {
                // Simnet is not supported in bitcoin crate, treat as testnet
                bitcoin::Network::Testnet
            }
        };

        // Verify the address is for the correct network
        let address = address.require_network(network).map_err(|_| {
            anyhow::anyhow!(
                "Fallback address {} is not valid for network {}",
                fallback_str,
                invoice_data.network
            )
        })?;

        // Convert to Fallback based on address type
        use bitcoin::address::AddressType;
        let fallback = match address.address_type() {
            Some(AddressType::P2pkh) => {
                // Extract pubkey hash from the script
                let script = address.script_pubkey();
                let bytes = script.as_bytes();
                if bytes.len() >= 23 && bytes[0] == 0x76 && bytes[1] == 0xa9 && bytes[2] == 0x14 {
                    let mut hash = [0u8; 20];
                    hash.copy_from_slice(&bytes[3..23]);
                    Fallback::PubKeyHash(PubkeyHash::from_byte_array(hash))
                } else {
                    bail!("Invalid P2PKH script")
                }
            }
            Some(AddressType::P2sh) => {
                // Extract script hash from the script
                let script = address.script_pubkey();
                let bytes = script.as_bytes();
                if bytes.len() >= 23 && bytes[0] == 0xa9 && bytes[1] == 0x14 {
                    let mut hash = [0u8; 20];
                    hash.copy_from_slice(&bytes[2..22]);
                    Fallback::ScriptHash(ScriptHash::from_byte_array(hash))
                } else {
                    bail!("Invalid P2SH script")
                }
            }
            Some(AddressType::P2wpkh) | Some(AddressType::P2wsh) | Some(AddressType::P2tr) => {
                // SegWit addresses
                let script = address.script_pubkey();
                let bytes = script.as_bytes();
                if bytes.len() >= 2 {
                    use bitcoin::WitnessVersion;
                    let version = if bytes[0] == 0 {
                        WitnessVersion::V0
                    } else if bytes[0] == 0x51 {
                        WitnessVersion::V1
                    } else {
                        bail!("Unsupported witness version")
                    };
                    let program = bytes[2..].to_vec();
                    Fallback::SegWitProgram { version, program }
                } else {
                    bail!("Invalid SegWit script")
                }
            }
            _ => bail!("Unsupported fallback address type"),
        };

        builder = builder.fallback(fallback);
    }

    // Add route hints
    for route in &invoice_data.routes {
        use lightning_invoice::{RouteHint, RouteHintHop, RoutingFees};

        let mut hints = Vec::new();
        for hop in route {
            let src_node_id = hop.src_node_id.inner().clone();

            let route_hop = RouteHintHop {
                src_node_id,
                short_channel_id: hop.short_channel_id,
                fees: RoutingFees {
                    base_msat: hop.fees.base_msat,
                    proportional_millionths: hop.fees.proportional_millionths,
                },
                cltv_expiry_delta: hop.cltv_expiry_delta,
                htlc_minimum_msat: hop.htlc_minimum_msat,
                htlc_maximum_msat: hop.htlc_maximum_msat,
            };
            hints.push(route_hop);
        }

        if !hints.is_empty() {
            // Create RouteHint from the hops
            let route_hint = RouteHint(hints);
            builder = builder.private_route(route_hint);
        }
    }

    // Build and sign the invoice
    let secp = Secp256k1::new();
    let signed_invoice = builder
        .build_signed(|hash| {
            secp.sign_ecdsa_recoverable(&Message::from_digest(*hash.as_ref()), &private_key)
        })
        .map_err(|e| anyhow::anyhow!("Failed to build and sign invoice: {:?}", e))?;

    Ok(signed_invoice.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_invoice() -> Result<()> {
        let invoice = "lnbc99810310n1pju0sy7pp555srgtgcg6t4jr4j5v0jysgee4zy6nr4msylnycfjezxm5w6t3csdy9wdmkzupq95s8xcmjd9c8gw3qx5cnyvrrvymrwvnrxgmrzd3cxsckxdf4v3jxgcmzx9jxgenpxserjenyxv6nzwf3vsmnyctxvsuxvdehvdnrswryxgcnzdf5ve3rjvph8q6njcqzxgxq97zvuqrzjqgwf02g2gy0l9vgdc25wxt0z72wjlfyagxlmk54ag9hyvrdsw37smapyqqqqqqqq2qqqqqqqqqqqqqqq9qsp59ge5l9ndweyes4ntfrws3a3tshpkqt8eysuxnt5pmucy9hvxthmq9qyyssqaqwn0j2jf2xvcv42yl9p0yaw4t6gcqld2t44cmnfud49dxgl3dnpnjpj75kaf22yuynqtc8uzmtuckzxvfunxnr405gud8cexc5axqqphlk58z";
        let output = decode_invoice(invoice)?;

        // Basic invoice properties
        assert_eq!(output.network, Network::Bitcoin);
        assert_eq!(output.amount_msats, Some(9981031000));
        assert_eq!(output.timestamp_millis, 1707589790000);

        // Payment details
        assert_eq!(
            output.payment_hash.to_hex(),
            "a520342d184697590eb2a31f224119cd444d4c75dc09f9930996446dd1da5c71"
        );
        assert_eq!(
            output.payment_secret.to_hex(),
            "2a334f966d764998566b48dd08f62b85c3602cf9243869ae81df3042dd865df6"
        );
        assert_eq!(
            output.destination.to_hex(),
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
            route.src_node_id.to_hex(),
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
    fn test_decode_lnurl() -> Result<()> {
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
    fn test_decode_invalid_lnurl() -> Result<()> {
        let invalid_lnurl = "not_an_lnurl";
        let result = decode_lnurl(invalid_lnurl);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_decode_invalid_invoice() -> Result<()> {
        let invalid_invoice = "not_an_invoice";
        let result = decode_invoice(invalid_invoice);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_parse_lightning_address() -> Result<()> {
        let address = "user@domain.com";
        let parts: Vec<&str> = address.split('@').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "user");
        assert_eq!(parts[1], "domain.com");
        Ok(())
    }

    #[test]
    fn test_invalid_lightning_address() -> Result<()> {
        let invalid_addresses = vec!["invalid", "user@domain@com"];

        for address in invalid_addresses {
            let parts: Vec<&str> = address.split('@').collect();
            assert_ne!(parts.len(), 2, "Address '{address}' should be invalid");
        }

        // Test addresses with empty parts
        let empty_part_addresses = vec!["user@", "@domain.com"];

        for address in empty_part_addresses {
            let parts: Vec<&str> = address.split('@').collect();
            assert_eq!(parts.len(), 2);
            assert!(
                parts[0].is_empty() || parts[1].is_empty(),
                "Address '{address}' should have empty part"
            );
        }
        Ok(())
    }

    #[test]
    fn test_encode_decode_invoice_roundtrip() -> Result<()> {
        use bitcoin::secp256k1::{Secp256k1, SecretKey};

        // Use a real private key and derive the public key from it
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let private_key_bytes = hex::decode(private_key_hex)?;
        let private_key = SecretKey::from_slice(&private_key_bytes)?;

        // Derive the public key from the private key
        let secp = Secp256k1::new();
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &private_key);
        let destination = hex::encode(public_key.serialize());

        // Create payment hash and secret
        let payment_hash = PaymentHash::from_slice(&hex::decode(
            "0001020304050607080910111213141516171819202122232425262728293031",
        )?)?;
        let payment_secret = PaymentSecret::from_slice(&hex::decode(
            "1111111111111111111111111111111111111111111111111111111111111111",
        )?)?;

        // Create timestamp
        let timestamp = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")?.with_timezone(&Utc);

        // Convert destination to PublicKey
        let destination = PublicKey::from_hex(&destination)?;

        // Create a test invoice data structure with matching destination
        let invoice_data = InvoiceOutput {
            network: Network::Bitcoin,
            amount_msats: Some(1000000),
            timestamp,
            timestamp_millis: 1704067200000,
            payment_hash,
            payment_secret,
            features: vec![],
            description: Some("Test invoice".to_string()),
            description_hash: None,
            destination,
            expiry_seconds: 3600,
            min_final_cltv_expiry: 18,
            fallback_addresses: vec![],
            routes: vec![],
        };

        // Encode the invoice
        let encoded = encode_invoice(&invoice_data, private_key_hex)?;

        // Verify it starts with lnbc (bitcoin mainnet)
        assert!(encoded.starts_with("lnbc"));

        // Decode it back
        let decoded = decode_invoice(&encoded)?;

        // Check key fields match
        assert_eq!(decoded.network, invoice_data.network);
        assert_eq!(decoded.amount_msats, invoice_data.amount_msats);
        assert_eq!(decoded.payment_hash, invoice_data.payment_hash);
        assert_eq!(decoded.payment_secret, invoice_data.payment_secret);
        assert_eq!(decoded.description, invoice_data.description);
        assert_eq!(decoded.expiry_seconds, invoice_data.expiry_seconds);
        assert_eq!(
            decoded.min_final_cltv_expiry,
            invoice_data.min_final_cltv_expiry
        );

        Ok(())
    }

    #[test]
    fn test_encode_invoice_testnet() -> Result<()> {
        use bitcoin::secp256k1::{Secp256k1, SecretKey};

        // Use a real private key and derive the public key from it
        let private_key_hex = "0202020202020202020202020202020202020202020202020202020202020202";
        let private_key_bytes = hex::decode(private_key_hex)?;
        let private_key = SecretKey::from_slice(&private_key_bytes)?;

        // Derive the public key from the private key
        let secp = Secp256k1::new();
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &private_key);
        let destination = PublicKey::from_slice(&public_key.serialize())?;

        // Create payment hash and secret
        let payment_hash = PaymentHash::from_slice(&hex::decode(
            "0001020304050607080910111213141516171819202122232425262728293031",
        )?)?;
        let payment_secret = PaymentSecret::from_slice(&hex::decode(
            "1111111111111111111111111111111111111111111111111111111111111111",
        )?)?;

        // Create description hash
        let description_hash = Some(Sha256Hash::from_slice(&hex::decode(
            "3132333435363738393031323334353637383930313233343536373839303132",
        )?)?);

        // Create timestamp
        let timestamp = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")?.with_timezone(&Utc);

        let invoice_data = InvoiceOutput {
            network: Network::Testnet,
            amount_msats: None, // No amount
            timestamp,
            timestamp_millis: 1704067200000,
            payment_hash,
            payment_secret,
            features: vec![],
            description: None,
            description_hash,
            destination,
            expiry_seconds: 7200,
            min_final_cltv_expiry: 144,
            fallback_addresses: vec![],
            routes: vec![],
        };

        let encoded = encode_invoice(&invoice_data, private_key_hex)?;

        // Verify it starts with lntb (bitcoin testnet)
        assert!(encoded.starts_with("lntb"));

        // Decode and verify
        let decoded = decode_invoice(&encoded)?;
        assert_eq!(decoded.network, Network::Testnet);
        assert_eq!(decoded.amount_msats, None);
        assert_eq!(decoded.description_hash, invoice_data.description_hash);

        Ok(())
    }
}
