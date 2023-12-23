mod websocket_client;
mod http_client;
mod orderbook;
mod messages;
mod saifutaku_config;

use url::Url;
use tokio::sync::Notify;
use std::sync::Arc;
use tokio::sync::mpsc::{self};
use orderbook::OrderBook;
use messages::{SnapShotUpdate, DepthUpdate, TradeUpdate};
use websocket_client::WebSocketClient;
use http_client::HttpClient;
use log::{info, error};
use saifutaku_config::SaifutakuConfig;
use config::{Config, File, ConfigError};

fn load_configuration() -> Result<SaifutakuConfig, ConfigError> {
    let mut settings = Config::default();
    settings.merge(File::with_name("Config"))?;
    let config = settings.try_into::<SaifutakuConfig>()?;
    config.validate().map_err(|e| ConfigError::Message(e.to_string()))?;
    Ok(config)
}

#[tokio::main]
async fn main() {

    env_logger::init();
    info!("Starting saifutaku livetester");
    let config = load_configuration().expect("Failed to load configuration");
    let shutdown_notify = Arc::new(Notify::new());
    
    let depth_url = Url::parse(&config.depth_url).expect("Failed to parse URL");
    let (depth_tx, mut depth_rx) = mpsc::channel(config.channel_size);
    let depth_shutdown_notify = shutdown_notify.clone();
    let mut depth_client: WebSocketClient<DepthUpdate> = WebSocketClient::new(depth_url, depth_tx, depth_shutdown_notify);

    tokio::spawn(async move {
        depth_client.connect().await;
    });
    
    let trade_url = Url::parse(&config.trade_url).expect("Failed to parse trade WebSocket URL");
    let (trade_tx, mut trade_rx) = mpsc::channel(config.channel_size);
    let trade_shutdown_notify = shutdown_notify.clone();
    let mut trade_client: WebSocketClient<TradeUpdate> = WebSocketClient::new(trade_url, trade_tx, trade_shutdown_notify);

    tokio::spawn(async move {
        trade_client.connect().await;
    });

    let (snapshot_tx, mut snapshot_rx) = mpsc::channel(config.channel_size);
    let snapshot_url = Url::parse(&config.snapshot_url).expect("Failed to parse URL");
    let snapshot_client_shutdown_notify = shutdown_notify.clone();
    let mut snapshot_client: HttpClient<SnapShotUpdate> = HttpClient::new(snapshot_url.to_string(), snapshot_tx, config.fetch_interval, snapshot_client_shutdown_notify);

    tokio::spawn(async move {
        snapshot_client.query_api().await;
    });
    
    let mut orderbook = OrderBook::new();

    // Set up the termination signal listener outside of the select block
    let mut termination_signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("Failed to set up termination signal listener");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C signal");
                break;
            },
            _ = termination_signal.recv() => {
                info!("Received termination signal");
                break;
            },
            update = snapshot_rx.recv() => {
                if let Some(update) = update {
                    orderbook.update_with_snapshot(update);
                } else {
                    error!("HTTP receiver channel closed unexpectedly");
                    break;
                }
            },
            update = depth_rx.recv() => {
                if let Some(update) = update {
                    orderbook.update(update);
                } else {
                    error!("WebSocket receiver channel closed unexpectedly");
                    break;
                }
            },
            update = trade_rx.recv() => {
                if let Some(update) = update {
                    orderbook.update_trade_stack(update);
                } else {
                    error!("WebSocket receiver channel closed unexpectedly");
                    break;
                }
            },
            else => break, // Break the loop if all channels are closed
        }
    }
        
    // Notify the clients to shut down
    shutdown_notify.notify_waiters(); // Send to all listeners!

    // Process remaining messages, if needed
    while let Some(update) = snapshot_rx.recv().await {
        orderbook.update_with_snapshot(update);
    }
    while let Some(update) = depth_rx.recv().await {
        orderbook.update(update);
    }
    while let Some(update) = trade_rx.recv().await {
        orderbook.update_trade_stack(update);
    }

    // Print the order book
    orderbook.print_orderbook(10, "Final state of the order book");
}
