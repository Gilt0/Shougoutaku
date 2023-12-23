use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Custom deserialization function for Decimal
/// Converts a string to a Decimal, returning an error if the string is not a valid representation.
fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<Decimal>().map_err(serde::de::Error::custom)
}

/// Custom serialization function for Decimal
/// Converts a Decimal to a string representation.
fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = value.to_string();
    serializer.serialize_str(&s)
}

/// Custom deserialization function for trade_id
/// Supports deserializing a trade_id from either a string or a number in JSON.
fn deserialize_trade_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(num) => Ok(num.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "Invalid type for trade_id: expected string or number, found {:?}",
            other
        ))),
    }
}

/// Represents the JSON message format for depth updates via WebSockets.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DepthUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id_in_event: u64,
    #[serde(rename = "u")]
    pub final_update_id_in_event: u64,
    /// List of bid values to update; each bid is represented as a pair of Decimals [price, quantity].
    #[serde(rename = "b")]
    pub bids_to_update: Vec<(Decimal, Decimal)>,
    /// List of ask values to update; each ask is represented as a pair of Decimals [price, quantity].
    #[serde(rename = "a")]
    pub asks_to_update: Vec<(Decimal, Decimal)>,
}

/// Represents the JSON message format for an order book snapshot update via HTTP.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SnapShotUpdate {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    /// List of bids; each bid is represented as a tuple of Decimals (price, quantity).
    pub bids: Vec<(Decimal, Decimal)>,
    /// List of asks; each ask is represented as a tuple of Decimals (price, quantity).
    pub asks: Vec<(Decimal, Decimal)>,
}

/// Represents the JSON message format for trade updates via WebSockets.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TradeUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t", deserialize_with = "deserialize_trade_id")]
    pub trade_id: String,
    #[serde(rename = "p", deserialize_with = "deserialize_decimal", serialize_with = "serialize_decimal")]
    pub price: Decimal,
    #[serde(rename = "q", deserialize_with = "deserialize_decimal", serialize_with = "serialize_decimal")]
    pub quantity: Decimal,
    #[serde(rename = "b")]
    pub buyer_order_id: u64,
    #[serde(rename = "a")]
    pub seller_order_id: u64,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "m")]
    pub is_market_maker: bool,
}
