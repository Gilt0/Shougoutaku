// saifutaku_config.rs
use serde::Deserialize;
use url::Url;
use std::error::Error;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct SaifutakuConfig {
    pub depth_url: String,
    pub trade_url: String,
    pub snapshot_url: String,
    pub channel_size: usize,
    pub fetch_interval: u64,
}

#[derive(Debug)]
pub struct ConfigValidationError {
    message: String,
}

impl ConfigValidationError {
    fn new(message: &str) -> Self {
        ConfigValidationError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Configuration validation error: {}", self.message)
    }
}

impl Error for ConfigValidationError {}

impl SaifutakuConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Validate HTTP URL
        if Url::parse(&self.snapshot_url).is_err() {
            return Err(ConfigValidationError::new("Invalid HTTP URL"));
        }

        // Validate websocket URL for depth updates
        if Url::parse(&self.depth_url).is_err() {
            return Err(ConfigValidationError::new("Invalid WebSocket URL"));
        }

        // Validate websocket URL fro trade updates
        if Url::parse(&self.trade_url).is_err() {
            return Err(ConfigValidationError::new("Invalid WebSocket URL"));
        }

        // Validate channel size
        if self.channel_size == 0 {
            return Err(ConfigValidationError::new("Channel size must be greater than 0"));
        }

        // Validate fetch interval
        if self.fetch_interval == 0 {
            return Err(ConfigValidationError::new("Fetch interval must be greater than 0"));
        }

        Ok(())
    }
}
