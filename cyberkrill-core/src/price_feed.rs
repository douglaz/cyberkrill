use anyhow::{Context, bail};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinSet;

type FeedResponse = (&'static str, anyhow::Result<Option<PriceQuote>>);

const MILLISATS_PER_BTC: f64 = 100_000_000_000.0;
const PRICE_SPREAD_WARN_THRESHOLD: f64 = 0.02;
// Current keyless public Fedi price endpoint; keep this in one place if it changes.
const FEDI_PRICE_FEED_URL: &str = "https://price-feed.dev.fedibtc.com/latest";

#[derive(Debug)]
struct FeedCollection {
    sources: Vec<PriceQuote>,
    issues: Vec<String>,
}

/// Aggregated BTC price across multiple feeds.
#[derive(Debug, Clone, Serialize)]
pub struct BtcPrice {
    /// ISO currency code, uppercase (e.g. "USD", "BRL").
    pub currency: String,
    /// Median price: how many units of `currency` equal 1 BTC.
    pub price_per_btc: f64,
    /// Per-feed quotes that contributed to the median.
    pub sources: Vec<PriceQuote>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceQuote {
    pub source: &'static str,
    pub price_per_btc: f64,
}

impl BtcPrice {
    /// Convert a fiat amount using this BTC price.
    pub fn amount_to_btc(&self, amount: f64) -> anyhow::Result<crate::AmountInput> {
        fiat_amount_to_btc_amount(amount, self.price_per_btc)
    }
}

fn normalize_fiat_currency(currency: &str) -> anyhow::Result<String> {
    let normalized = currency.trim().to_ascii_uppercase();
    if normalized.len() != 3 || !normalized.chars().all(|c| c.is_ascii_alphabetic()) {
        bail!("Invalid fiat currency code: '{currency}'");
    }
    Ok(normalized)
}

/// Fetch BTC price for `currency` from all configured feeds in parallel and return the median.
///
/// This contacts third-party HTTPS price feeds and does not reuse Bitcoin Core proxy settings.
///
/// Errors if fewer than 3 feeds succeed (the 3-source quorum makes the median
/// a true middle value, so a single bad/poisoned feed cannot swing the result).
pub async fn fetch_btc_price(currency: &str) -> anyhow::Result<BtcPrice> {
    let currency = normalize_fiat_currency(currency)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("cyberkrill/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Invalid BTC price feed client configuration")?;

    let mut feeds = JoinSet::new();

    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("coingecko", fetch_coingecko(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("coinbase", fetch_coinbase(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move {
            (
                "blockchain.info",
                fetch_blockchain_info(&client, &currency).await,
            )
        });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("kraken", fetch_kraken(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("fedi", fetch_fedi(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("cex.io", fetch_cex_io(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("bitstamp", fetch_bitstamp(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("yadio", fetch_yadio(&client, &currency).await) });
    }
    {
        let client = client.clone();
        let currency = currency.clone();
        feeds.spawn(async move { ("gemini", fetch_gemini(&client, &currency).await) });
    }

    let FeedCollection { sources, issues } = collect_feed_quotes(feeds, &currency).await;
    let result = aggregate_btc_price(currency.clone(), sources);
    if issues.is_empty() {
        result
    } else {
        let issue_summary = format!(
            "BTC price feed issues for {currency}: {issues}",
            issues = issues.join("; ")
        );
        match result {
            Ok(price) => {
                // Trust signal — must reach stderr regardless of RUST_LOG.
                eprintln!("[price-feed] {issue_summary}");
                Ok(price)
            }
            Err(error) => Err(error).context(issue_summary),
        }
    }
}

fn aggregate_btc_price(currency: String, mut sources: Vec<PriceQuote>) -> anyhow::Result<BtcPrice> {
    // Require at least 3 sources so the median is a true middle value: with 2
    // sources the "median" is the arithmetic mean and a single bad/poisoned
    // feed can swing the result by ~50% of the spread.
    if sources.len() < 3 {
        let feed_word = if sources.len() == 1 { "feed" } else { "feeds" };
        bail!(
            "Only {source_count} BTC price {feed_word} responded for {currency}; need at least 3 to compute a manipulation-resistant median",
            source_count = sources.len()
        );
    }

    sources.sort_by(|left, right| {
        left.price_per_btc
            .total_cmp(&right.price_per_btc)
            .then_with(|| left.source.cmp(right.source))
    });
    let price_per_btc = median(
        sources
            .iter()
            .map(|quote| quote.price_per_btc)
            .collect::<Vec<_>>(),
    )
    .context("At least one BTC price source is required")?;
    warn_if_price_spread_is_high(&currency, price_per_btc, &sources);

    Ok(BtcPrice {
        currency,
        price_per_btc,
        sources,
    })
}

fn warn_if_price_spread_is_high(currency: &str, median_price: f64, sources: &[PriceQuote]) {
    let Some(min) = sources.first() else {
        return;
    };
    let Some(max) = sources.last() else {
        return;
    };

    let spread = max.price_per_btc / min.price_per_btc - 1.0;
    if spread > PRICE_SPREAD_WARN_THRESHOLD {
        // Trust signal — must reach stderr regardless of RUST_LOG.
        eprintln!(
            "[price-feed] BTC price feed spread for {currency} is {spread_percent:.2}% \
             ({min_source}={min_price:.2}, {max_source}={max_price:.2}); \
             using median BTC/{currency}={median_price:.2}",
            spread_percent = spread * 100.0,
            min_source = min.source,
            min_price = min.price_per_btc,
            max_source = max.source,
            max_price = max.price_per_btc
        );
    }
}

async fn collect_feed_quotes(mut feeds: JoinSet<FeedResponse>, currency: &str) -> FeedCollection {
    let mut sources = Vec::new();
    let mut issues = Vec::new();
    while let Some(result) = feeds.join_next().await {
        match result {
            Ok((_source, Ok(Some(quote)))) => sources.push(quote),
            Ok((source, Ok(None))) => {
                issues.push(format!("{source}: no {currency} quote"));
                tracing::debug!("BTC price feed {source} did not return {currency} quote");
            }
            Ok((source, Err(error))) => {
                issues.push(format!("{source}: {error:#}"));
                tracing::debug!("BTC price feed {source} failed for {currency}: {error:#}");
            }
            Err(error) => {
                issues.push(format!("feed task failed: {error}"));
                tracing::debug!("BTC price feed task failed for {currency}: {error}");
            }
        }
    }
    FeedCollection { sources, issues }
}

fn fiat_amount_to_btc_amount(
    amount: f64,
    price_per_btc: f64,
) -> anyhow::Result<crate::AmountInput> {
    let millisats = fiat_amount_to_millisats(amount, price_per_btc)?;
    Ok(crate::AmountInput::from_millisats(millisats))
}

fn fiat_amount_to_millisats(amount: f64, price_per_btc: f64) -> anyhow::Result<u64> {
    if !amount.is_finite() || amount < 0.0 {
        bail!("Invalid fiat amount: {amount}");
    }
    if !price_per_btc.is_finite() || price_per_btc <= 0.0 {
        bail!("Invalid BTC price: {price_per_btc}");
    }

    let rounded_millisats = (amount / price_per_btc * MILLISATS_PER_BTC).round();
    if !rounded_millisats.is_finite()
        || rounded_millisats < 0.0
        || rounded_millisats >= u64::MAX as f64
    {
        bail!("Converted fiat amount is outside the supported range");
    }

    let millisats = rounded_millisats as u64;
    if amount > 0.0 && millisats == 0 {
        bail!("Converted non-zero fiat amount is less than 1 msat");
    }
    Ok(millisats)
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Some(values[middle])
    }
}

async fn fetch_coingecko(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let lower = currency.to_ascii_lowercase();
    let url =
        format!("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies={lower}");
    let body = client
        .get(url)
        .send()
        .await
        .context("CoinGecko request failed")?
        .error_for_status()
        .context("CoinGecko returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read CoinGecko response")?;

    Ok(
        parse_coingecko_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "coingecko",
            price_per_btc,
        }),
    )
}

async fn fetch_coinbase(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let body = client
        .get("https://api.coinbase.com/v2/exchange-rates?currency=BTC")
        .send()
        .await
        .context("Coinbase request failed")?
        .error_for_status()
        .context("Coinbase returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Coinbase response")?;

    Ok(
        parse_coinbase_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "coinbase",
            price_per_btc,
        }),
    )
}

async fn fetch_blockchain_info(
    client: &Client,
    currency: &str,
) -> anyhow::Result<Option<PriceQuote>> {
    let body = client
        .get("https://blockchain.info/ticker")
        .send()
        .await
        .context("Blockchain.info request failed")?
        .error_for_status()
        .context("Blockchain.info returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Blockchain.info response")?;

    Ok(
        parse_blockchain_info_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "blockchain.info",
            price_per_btc,
        }),
    )
}

async fn fetch_kraken(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let url = format!("https://api.kraken.com/0/public/Ticker?pair=XBT{currency}");
    let body = client
        .get(url)
        .send()
        .await
        .context("Kraken request failed")?
        .error_for_status()
        .context("Kraken returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Kraken response")?;

    Ok(
        parse_kraken_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "kraken",
            price_per_btc,
        }),
    )
}

async fn fetch_fedi(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let body = client
        .get(FEDI_PRICE_FEED_URL)
        .send()
        .await
        .context("Fedi request failed")?
        .error_for_status()
        .context("Fedi returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Fedi response")?;

    Ok(
        parse_fedi_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "fedi",
            price_per_btc,
        }),
    )
}

async fn fetch_cex_io(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let url = format!("https://cex.io/api/ticker/BTC/{currency}");
    let response = client
        .get(url)
        .send()
        .await
        .context("CEX.IO request failed")?;

    // 404 means the pair isn't listed; surface other non-success codes
    // (429/5xx/...) so the aggregator records them as real issues.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let body = response
        .error_for_status()
        .context("CEX.IO returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read CEX.IO response")?;

    Ok(
        parse_cex_io_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "cex.io",
            price_per_btc,
        }),
    )
}

async fn fetch_bitstamp(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let lower = currency.to_ascii_lowercase();
    let url = format!("https://www.bitstamp.net/api/v2/ticker/btc{lower}");
    let response = client
        .get(url)
        .send()
        .await
        .context("Bitstamp request failed")?;

    // Bitstamp returns 404 for unsupported pairs; surface other non-success
    // codes (429/5xx/...) so the aggregator records them as real issues.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let body = response
        .error_for_status()
        .context("Bitstamp returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Bitstamp response")?;

    Ok(
        parse_bitstamp_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "bitstamp",
            price_per_btc,
        }),
    )
}

async fn fetch_yadio(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let url = format!("https://api.yadio.io/convert/1/BTC/{currency}");
    let response = client
        .get(url)
        .send()
        .await
        .context("Yadio request failed")?;

    // 404 means the currency isn't quoted; surface other non-success codes
    // (429/5xx/...) so the aggregator records them as real issues.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let body = response
        .error_for_status()
        .context("Yadio returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Yadio response")?;

    Ok(
        parse_yadio_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "yadio",
            price_per_btc,
        }),
    )
}

async fn fetch_gemini(client: &Client, currency: &str) -> anyhow::Result<Option<PriceQuote>> {
    let lower = currency.to_ascii_lowercase();
    let url = format!("https://api.gemini.com/v1/pubticker/btc{lower}");
    let response = client
        .get(url)
        .send()
        .await
        .context("Gemini request failed")?;

    // Gemini returns 404 for unknown pairs; surface other non-success codes
    // (429/5xx/...) so the aggregator records them as real issues.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let body = response
        .error_for_status()
        .context("Gemini returned an unsuccessful status")?
        .text()
        .await
        .context("Failed to read Gemini response")?;

    Ok(
        parse_gemini_price(&body, currency)?.map(|price_per_btc| PriceQuote {
            source: "gemini",
            price_per_btc,
        }),
    )
}

fn parse_coingecko_price(json: &str, currency: &str) -> anyhow::Result<Option<f64>> {
    let value: Value = serde_json::from_str(json).context("Invalid CoinGecko JSON")?;
    let lower = currency.to_ascii_lowercase();
    let Some(price) = value.get("bitcoin").and_then(|bitcoin| bitcoin.get(&lower)) else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "CoinGecko price")?))
}

fn parse_coinbase_price(json: &str, currency: &str) -> anyhow::Result<Option<f64>> {
    let value: Value = serde_json::from_str(json).context("Invalid Coinbase JSON")?;
    let Some(price) = value
        .get("data")
        .and_then(|data| data.get("rates"))
        .and_then(|rates| rates.get(currency))
    else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "Coinbase rate")?))
}

fn parse_blockchain_info_price(json: &str, currency: &str) -> anyhow::Result<Option<f64>> {
    let value: Value = serde_json::from_str(json).context("Invalid Blockchain.info JSON")?;
    let Some(price) = value
        .get(currency)
        .and_then(|currency_data| currency_data.get("last"))
    else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(
        price,
        "Blockchain.info last price",
    )?))
}

fn parse_kraken_price(json: &str, currency: &str) -> anyhow::Result<Option<f64>> {
    let value: Value = serde_json::from_str(json).context("Invalid Kraken JSON")?;
    if value
        .get("error")
        .and_then(|errors| errors.as_array())
        .is_some_and(|errors| !errors.is_empty())
    {
        return Ok(None);
    }

    let Some(result) = value.get("result").and_then(|result| result.as_object()) else {
        return Ok(None);
    };

    let mut candidates = Vec::new();
    for (pair, ticker) in result {
        let Some(price) = ticker
            .get("c")
            .and_then(|close| close.as_array())
            .and_then(|close| close.first())
        else {
            continue;
        };

        candidates.push((
            pair.as_str(),
            parse_positive_price(price, "Kraken close price")?,
        ));
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    if candidates.len() == 1 {
        return Ok(Some(candidates[0].1));
    }

    candidates.sort_by(|left, right| left.0.cmp(right.0));
    if let Some((_, price)) = candidates.iter().find(|(pair, _)| pair.ends_with(currency)) {
        return Ok(Some(*price));
    }

    Ok(None)
}

fn parse_fedi_price(json: &str, currency: &str) -> anyhow::Result<Option<f64>> {
    let value: Value = serde_json::from_str(json).context("Invalid Fedi JSON")?;
    let Some(prices) = value.get("prices").and_then(|prices| prices.as_object()) else {
        return Ok(None);
    };
    let Some(btc_usd) = fedi_pair_rate(prices, "BTC/USD")? else {
        return Ok(None);
    };

    if currency == "USD" {
        return Ok(Some(btc_usd));
    }

    let pair = format!("{currency}/USD");
    let Some(fiat_usd) = fedi_pair_rate(prices, &pair)? else {
        return Ok(None);
    };

    // Fedi quotes are oriented as X/USD. This cross-rate uses the BTC/USD
    // and fiat/USD rates from the same response as one median input source.
    Ok(Some(validate_positive_price(
        btc_usd / fiat_usd,
        "Fedi cross-rate",
    )?))
}

fn fedi_pair_rate(
    prices: &serde_json::Map<String, Value>,
    pair: &str,
) -> anyhow::Result<Option<f64>> {
    let Some(rate) = prices.get(pair).and_then(|pair_data| pair_data.get("rate")) else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(rate, "Fedi rate")?))
}

fn parse_cex_io_price(json: &str, _currency: &str) -> anyhow::Result<Option<f64>> {
    // The currency is encoded in the request URL, so the response body is
    // implicitly for that pair. We only need to detect the unsupported-pair
    // shape and read the `last` price.
    let value: Value = serde_json::from_str(json).context("Invalid CEX.IO JSON")?;
    if value.get("error").is_some() {
        return Ok(None);
    }

    let Some(price) = value.get("last") else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "CEX.IO last price")?))
}

fn parse_bitstamp_price(json: &str, _currency: &str) -> anyhow::Result<Option<f64>> {
    // The currency is encoded in the request URL (btc<currency>), so the
    // response body is implicitly for that pair.
    let value: Value = serde_json::from_str(json).context("Invalid Bitstamp JSON")?;
    let Some(price) = value.get("last") else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "Bitstamp last price")?))
}

fn parse_yadio_price(json: &str, _currency: &str) -> anyhow::Result<Option<f64>> {
    // The currency is encoded in the request URL, so the top-level `rate`
    // field is implicitly for that pair.
    let value: Value = serde_json::from_str(json).context("Invalid Yadio JSON")?;
    if value.get("error").is_some() {
        return Ok(None);
    }

    let Some(price) = value.get("rate") else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "Yadio rate")?))
}

fn parse_gemini_price(json: &str, _currency: &str) -> anyhow::Result<Option<f64>> {
    // The currency is encoded in the request URL (btc<currency>), so the
    // response body is implicitly for that pair.
    let value: Value = serde_json::from_str(json).context("Invalid Gemini JSON")?;
    if value
        .get("result")
        .and_then(|result| result.as_str())
        .is_some_and(|result| result.eq_ignore_ascii_case("error"))
    {
        return Ok(None);
    }

    let Some(price) = value.get("last") else {
        return Ok(None);
    };

    Ok(Some(parse_positive_price(price, "Gemini last price")?))
}

fn parse_positive_price(value: &Value, label: &str) -> anyhow::Result<f64> {
    let price = if let Some(price) = value.as_f64() {
        price
    } else if let Some(price) = value.as_str() {
        price
            .parse::<f64>()
            .with_context(|| format!("{label} is not a number"))?
    } else {
        bail!("{label} is not a number");
    };

    validate_positive_price(price, label)
}

fn validate_positive_price(price: f64, label: &str) -> anyhow::Result<f64> {
    if !price.is_finite() || price <= 0.0 {
        bail!("{label} must be positive");
    }

    Ok(price)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        let delta = (actual - expected).abs();
        assert!(
            delta < 0.000_001,
            "actual {actual} was not close to expected {expected}"
        );
    }

    #[test]
    fn median_handles_ordering_and_even_counts() -> anyhow::Result<()> {
        assert_close(median(vec![3.0, 1.0, 2.0]).context("odd median")?, 2.0);
        assert_close(
            median(vec![4.0, 1.0, 2.0, 3.0]).context("even median")?,
            2.5,
        );
        assert_close(median(vec![8.0, 2.0]).context("two-value median")?, 5.0);
        assert_close(
            median(vec![10.0, 1.0, 9.0, 2.0, 8.0]).context("ordered median")?,
            8.0,
        );
        assert!(median(Vec::new()).is_none());
        Ok(())
    }

    #[test]
    fn normalizes_three_letter_fiat_currency_codes() -> anyhow::Result<()> {
        assert_eq!(normalize_fiat_currency(" usd ")?, "USD");
        assert_eq!(normalize_fiat_currency("xau")?, "XAU");
        assert!(normalize_fiat_currency("USDC").is_err());
        assert!(normalize_fiat_currency("12A").is_err());
        Ok(())
    }

    #[test]
    fn fiat_conversion_rounds_to_nearest_millisat() -> anyhow::Result<()> {
        assert_eq!(fiat_amount_to_millisats(100.0, 95_000.0)?, 105_263_158);
        assert_eq!(
            fiat_amount_to_millisats(50_000.0, 100_000.0)?,
            50_000_000_000
        );

        let price = BtcPrice {
            currency: "USD".to_string(),
            price_per_btc: 95_000.0,
            sources: vec![
                PriceQuote {
                    source: "feed-a",
                    price_per_btc: 94_900.0,
                },
                PriceQuote {
                    source: "feed-b",
                    price_per_btc: 95_100.0,
                },
            ],
        };
        let amount = price.amount_to_btc(100.0)?;
        assert_eq!(amount.as_sat(), 105_263);
        assert_eq!(amount.as_millisats(), 105_263_158);

        let error = fiat_amount_to_millisats(100.0, 0.0)
            .err()
            .context("zero price should fail")?
            .to_string();
        assert!(error.contains("Invalid BTC price"), "{error}");
        Ok(())
    }

    #[test]
    fn fiat_conversion_rejects_nonzero_amounts_that_round_to_zero_millisats() -> anyhow::Result<()>
    {
        let error = fiat_amount_to_millisats(0.00000001, 100_000.0)
            .err()
            .context("non-zero fiat amount below 1 msat should fail")?
            .to_string();

        assert!(
            error.contains("Converted non-zero fiat amount is less than 1 msat"),
            "{error}"
        );
        assert_eq!(fiat_amount_to_millisats(0.0, 100_000.0)?, 0);
        Ok(())
    }

    #[test]
    fn fiat_conversion_rejects_integer_overflow_after_float_rounding() -> anyhow::Result<()> {
        let amount_that_would_overflow = u64::MAX as f64 / MILLISATS_PER_BTC + 1.0;
        let error = fiat_amount_to_millisats(amount_that_would_overflow, 1.0)
            .err()
            .context("overflow-sized conversion should fail")?
            .to_string();

        assert!(
            error.contains("Converted fiat amount is outside the supported range"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn aggregate_requires_at_least_three_sources() -> anyhow::Result<()> {
        let single_source_error = aggregate_btc_price(
            "USD".to_string(),
            vec![PriceQuote {
                source: "coingecko",
                price_per_btc: 100_000.0,
            }],
        )
        .err()
        .context("single source should fail")?
        .to_string();

        assert!(
            single_source_error.contains("Only 1 BTC price feed responded for USD"),
            "{single_source_error}"
        );

        let two_source_error = aggregate_btc_price(
            "USD".to_string(),
            vec![
                PriceQuote {
                    source: "coingecko",
                    price_per_btc: 100_000.0,
                },
                PriceQuote {
                    source: "coinbase",
                    price_per_btc: 100_500.0,
                },
            ],
        )
        .err()
        .context("two sources should still fail")?
        .to_string();

        assert!(
            two_source_error.contains("Only 2 BTC price feeds responded for USD"),
            "{two_source_error}"
        );
        Ok(())
    }

    #[test]
    fn aggregate_sorts_sources_and_uses_median() -> anyhow::Result<()> {
        let price = aggregate_btc_price(
            "USD".to_string(),
            vec![
                PriceQuote {
                    source: "z-feed",
                    price_per_btc: 102_000.0,
                },
                PriceQuote {
                    source: "a-feed",
                    price_per_btc: 100_000.0,
                },
                PriceQuote {
                    source: "m-feed",
                    price_per_btc: 101_000.0,
                },
            ],
        )?;

        assert_eq!(price.currency, "USD");
        assert_close(price.price_per_btc, 101_000.0);
        assert_eq!(
            price
                .sources
                .iter()
                .map(|quote| quote.source)
                .collect::<Vec<_>>(),
            ["a-feed", "m-feed", "z-feed"]
        );
        Ok(())
    }

    #[tokio::test]
    async fn collect_feed_quotes_drops_missing_errors_and_join_errors() -> anyhow::Result<()> {
        let mut feeds: JoinSet<FeedResponse> = JoinSet::new();
        feeds.spawn(async {
            (
                "coingecko",
                Ok(Some(PriceQuote {
                    source: "coingecko",
                    price_per_btc: 100_000.0,
                })),
            )
        });
        feeds.spawn(async { ("coinbase", Ok(None)) });
        feeds.spawn(async { ("kraken", Err(anyhow::anyhow!("boom"))) });
        feeds.spawn(async { panic!("feed task panicked") });

        let collection = collect_feed_quotes(feeds, "USD").await;

        assert_eq!(collection.sources.len(), 1);
        assert_eq!(collection.sources[0].source, "coingecko");
        assert_close(collection.sources[0].price_per_btc, 100_000.0);
        assert_eq!(collection.issues.len(), 3);
        assert!(
            collection
                .issues
                .iter()
                .any(|issue| issue.contains("coinbase: no USD quote"))
        );
        assert!(
            collection
                .issues
                .iter()
                .any(|issue| issue.contains("kraken: boom"))
        );
        assert!(
            collection
                .issues
                .iter()
                .any(|issue| issue.contains("feed task failed"))
        );
        Ok(())
    }

    #[test]
    fn parses_coingecko_response() -> anyhow::Result<()> {
        let json = r#"{"bitcoin":{"usd":104523.73,"brl":593424.10}}"#;

        assert_close(
            parse_coingecko_price(json, "USD")?.context("USD quote")?,
            104523.73,
        );
        assert_close(
            parse_coingecko_price(json, "BRL")?.context("BRL quote")?,
            593424.10,
        );
        Ok(())
    }

    #[test]
    fn parses_coinbase_response() -> anyhow::Result<()> {
        let json = r#"{
            "data": {
                "currency": "BTC",
                "rates": {
                    "USD": "104490.12",
                    "BRL": "593000.45"
                }
            }
        }"#;

        assert_close(
            parse_coinbase_price(json, "USD")?.context("USD quote")?,
            104490.12,
        );
        assert_close(
            parse_coinbase_price(json, "BRL")?.context("BRL quote")?,
            593000.45,
        );
        Ok(())
    }

    #[test]
    fn parses_blockchain_info_response() -> anyhow::Result<()> {
        let json = r#"{
            "USD": {"15m": 104400.50, "last": 104400.50, "buy": 104400.50, "sell": 104400.50, "symbol": "$"},
            "BRL": {"15m": 592900.25, "last": 592900.25, "buy": 592900.25, "sell": 592900.25, "symbol": "R$"}
        }"#;

        assert_close(
            parse_blockchain_info_price(json, "USD")?.context("USD quote")?,
            104400.50,
        );
        assert_close(
            parse_blockchain_info_price(json, "BRL")?.context("BRL quote")?,
            592900.25,
        );
        Ok(())
    }

    #[test]
    fn parses_kraken_response_without_assuming_pair_key() -> anyhow::Result<()> {
        let usd_json = r#"{
            "error": [],
            "result": {
                "XXBTZUSD": {
                    "a": ["104600.00000", "1", "1.000"],
                    "b": ["104599.90000", "1", "1.000"],
                    "c": ["104555.12000", "0.00400000"]
                }
            }
        }"#;
        let aud_json = r#"{
            "error": [],
            "result": {
                "XBTAUD": {
                    "a": ["160100.00000", "1", "1.000"],
                    "b": ["160099.90000", "1", "1.000"],
                    "c": ["160055.99000", "0.01000000"]
                }
            }
        }"#;

        assert_close(
            parse_kraken_price(usd_json, "USD")?.context("USD quote")?,
            104555.12,
        );
        assert_close(
            parse_kraken_price(aud_json, "AUD")?.context("AUD quote")?,
            160055.99,
        );
        Ok(())
    }

    #[test]
    fn parses_kraken_response_with_extra_result_fields() -> anyhow::Result<()> {
        let json = r#"{
            "error": [],
            "result": {
                "metadata": {"note": "ignored"},
                "XXBTZUSD": {"c": ["104555.12000", "0.00400000"]}
            }
        }"#;

        assert_close(
            parse_kraken_price(json, "USD")?.context("USD quote")?,
            104555.12,
        );
        Ok(())
    }

    #[test]
    fn kraken_parser_uses_matching_currency_when_multiple_tickers_are_present() -> anyhow::Result<()>
    {
        let json = r#"{
            "error": [],
            "result": {
                "XXBTZEUR": {"c": ["95000.00000", "0.10000000"]},
                "XXBTZUSD": {"c": ["104555.12000", "0.00400000"]}
            }
        }"#;

        assert_close(
            parse_kraken_price(json, "USD")?.context("USD quote")?,
            104555.12,
        );
        Ok(())
    }

    #[test]
    fn kraken_parser_uses_single_ticker_without_assuming_pair_key() -> anyhow::Result<()> {
        let json = r#"{
            "error": [],
            "result": {
                "UNEXPECTEDKEY": {"c": ["95000.00000", "0.10000000"]}
            }
        }"#;

        assert_close(
            parse_kraken_price(json, "USD")?.context("single quote")?,
            95000.0,
        );
        Ok(())
    }

    #[test]
    fn parses_fedi_response_orientation() -> anyhow::Result<()> {
        let json = r#"{
            "prices": {
                "BTC/USD": {"rate": 104000.00, "timestamp": 1760000000},
                "BRL/USD": {"rate": 0.18, "timestamp": 1760000000},
                "EUR/USD": {"rate": 1.08, "timestamp": 1760000000}
            }
        }"#;

        assert_close(
            parse_fedi_price(json, "USD")?.context("USD quote")?,
            104000.00,
        );
        assert_close(
            parse_fedi_price(json, "BRL")?.context("BRL quote")?,
            104000.00 / 0.18,
        );
        Ok(())
    }

    #[test]
    fn rejects_zero_fedi_cross_rate() -> anyhow::Result<()> {
        let json = r#"{
            "prices": {
                "BTC/USD": {"rate": 104000.00, "timestamp": 1760000000},
                "BRL/USD": {"rate": 0.0, "timestamp": 1760000000}
            }
        }"#;

        let error = parse_fedi_price(json, "BRL")
            .err()
            .context("zero Fedi fiat rate should fail")?
            .to_string();
        assert!(error.contains("Fedi rate must be positive"), "{error}");
        Ok(())
    }

    #[test]
    fn fedi_parser_drops_source_without_btc_usd() -> anyhow::Result<()> {
        let json = r#"{
            "prices": {
                "BRL/USD": {"rate": 0.18, "timestamp": 1760000000}
            }
        }"#;

        assert!(parse_fedi_price(json, "BRL")?.is_none());
        assert!(parse_fedi_price(json, "USD")?.is_none());
        Ok(())
    }

    #[test]
    fn parses_cex_io_response() -> anyhow::Result<()> {
        let json = r#"{
            "timestamp": "1760000000",
            "low": "66000.00",
            "high": "68000.00",
            "last": "67234.12",
            "volume": "12.34",
            "bid": 67230.00,
            "ask": 67240.00
        }"#;

        assert_close(
            parse_cex_io_price(json, "USD")?.context("USD quote")?,
            67234.12,
        );
        Ok(())
    }

    #[test]
    fn cex_io_parser_returns_none_for_error_body() -> anyhow::Result<()> {
        let json = r#"{"error": "Pair is disabled"}"#;
        assert!(parse_cex_io_price(json, "ZZZ")?.is_none());
        Ok(())
    }

    #[test]
    fn cex_io_parser_returns_none_when_last_is_missing() -> anyhow::Result<()> {
        let json = r#"{"bid": 1.0, "ask": 1.1}"#;
        assert!(parse_cex_io_price(json, "USD")?.is_none());
        Ok(())
    }

    #[test]
    fn parses_bitstamp_response() -> anyhow::Result<()> {
        let json = r#"{
            "last": "67234.10",
            "bid": "67230.00",
            "ask": "67240.00",
            "open": "67000.00",
            "high": "68000.00",
            "low": "66500.00",
            "volume": "100.5",
            "timestamp": "1760000000"
        }"#;

        assert_close(
            parse_bitstamp_price(json, "USD")?.context("USD quote")?,
            67234.10,
        );
        Ok(())
    }

    #[test]
    fn bitstamp_parser_returns_none_when_last_is_missing() -> anyhow::Result<()> {
        let json = r#"{"bid": "1.0", "ask": "1.1"}"#;
        assert!(parse_bitstamp_price(json, "USD")?.is_none());
        Ok(())
    }

    #[test]
    fn parses_yadio_response() -> anyhow::Result<()> {
        let json = r#"{
            "rate": 67234.12,
            "from": "BTC",
            "to": "USD",
            "request": {"amount": 1, "from": "BTC", "to": "USD"},
            "timestamp": 1760000000
        }"#;

        assert_close(
            parse_yadio_price(json, "USD")?.context("USD quote")?,
            67234.12,
        );
        Ok(())
    }

    #[test]
    fn yadio_parser_returns_none_for_error_body() -> anyhow::Result<()> {
        let json = r#"{"error": "Currency not supported"}"#;
        assert!(parse_yadio_price(json, "ZZZ")?.is_none());
        Ok(())
    }

    #[test]
    fn yadio_parser_returns_none_when_rate_is_missing() -> anyhow::Result<()> {
        let json = r#"{"from": "BTC", "to": "USD"}"#;
        assert!(parse_yadio_price(json, "USD")?.is_none());
        Ok(())
    }

    #[test]
    fn parses_gemini_response() -> anyhow::Result<()> {
        let json = r#"{
            "ask": "67240.00",
            "bid": "67230.00",
            "last": "67234.65",
            "volume": {"BTC": "100.5", "USD": "6700000.00", "timestamp": 1760000000}
        }"#;

        assert_close(
            parse_gemini_price(json, "USD")?.context("USD quote")?,
            67234.65,
        );
        Ok(())
    }

    #[test]
    fn gemini_parser_returns_none_for_error_body() -> anyhow::Result<()> {
        let json = r#"{"result": "error", "reason": "InvalidSymbol", "message": "Unknown pair"}"#;
        assert!(parse_gemini_price(json, "ZZZ")?.is_none());
        Ok(())
    }

    #[test]
    fn gemini_parser_returns_none_when_last_is_missing() -> anyhow::Result<()> {
        let json = r#"{"ask": "1.0", "bid": "1.1"}"#;
        assert!(parse_gemini_price(json, "USD")?.is_none());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn live_fetch_btc_price() -> anyhow::Result<()> {
        let price = fetch_btc_price("USD").await?;
        println!(
            "BTC/{currency} median = {price_per_btc} from {source_count} feeds: {sources}",
            currency = price.currency,
            price_per_btc = price.price_per_btc,
            source_count = price.sources.len(),
            sources = price
                .sources
                .iter()
                .map(|quote| quote.source)
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert!(price.sources.len() >= 2);
        Ok(())
    }
}
