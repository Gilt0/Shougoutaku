mod messages;
mod orderbook;
mod trade_matcher;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use log::{info, debug, error};
use rust_decimal::Decimal;
use std::error::Error;
use std::fmt;


use messages::{SnapShotUpdate, TradeUpdate, DepthUpdate};
use orderbook::OrderBook;
use trade_matcher::{TradeType, TradeMatcher};

type Reader = io::Lines<io::BufReader<File>>;

#[derive(Debug)]
struct TradeReadError {
    message: String,
}

impl TradeReadError {
    fn new(msg: &str) -> TradeReadError {
        TradeReadError { message: msg.to_string() }
    }
}

impl fmt::Display for TradeReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TradeReadError: {}", self.message)
    }
}

impl Error for TradeReadError {}

struct PreviousTradeInfo {
    prev_event_time: u64,
    prev_price: Decimal,
    prev_quantity: Decimal,
    prev_trade_id: String,
}

impl PreviousTradeInfo {
    pub fn new() -> Self {
        Self {
            prev_event_time: 0,
            prev_price: Decimal::new(0, 0),
            prev_quantity: Decimal::new(0, 0),
            prev_trade_id: String::from(""),
        }
    }
}

fn next_trades(trade_reader: &mut Reader, prev_trade_info: &mut PreviousTradeInfo, trade_matcher: &mut TradeMatcher, trade_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(Ok(line)) = trade_reader.next() {
        let mut trade_update: TradeUpdate = serde_json::from_str(&line)?;
        debug!("{} {:?}", trade_type, trade_update);
        // Compare current and previous values
        if trade_update.event_time == prev_trade_info.prev_event_time && trade_update.price == prev_trade_info.prev_price {
            trade_update.quantity += prev_trade_info.prev_quantity;
            trade_update.trade_id += "-";
            trade_update.trade_id += &prev_trade_info.prev_trade_id;
        }
        // Update the variables with current values for next iteration
        prev_trade_info.prev_event_time = trade_update.event_time;
        prev_trade_info.prev_price = trade_update.price.clone();
        prev_trade_info.prev_quantity = trade_update.quantity.clone();
        prev_trade_info.prev_trade_id = trade_update.trade_id.clone();
        trade_matcher.add_trade(trade_update);
        return Ok(());
    }
    Err(Box::new(TradeReadError::new("No more trades to read")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Matching Engine");
    env_logger::init();
    // Load the snapshot from the specified path
    let snapshot_path = Path::new("data/capture/btcusdt/1702798595534677/snapshot.txt");
    let snapshot_file = File::open(snapshot_path)?;
    let snapshot_reader = io::BufReader::new(snapshot_file);
    let snapshot: SnapShotUpdate = serde_json::from_reader(snapshot_reader)?;

    debug!("Snapshot loaded: {:?}", snapshot);

    // Open the files using the specified paths
    let ask_trade_file = File::open("data/capture/btcusdt/1702798595534677/ask_trade.txt")?;
    let bid_trade_file = File::open("data/capture/btcusdt/1702798595534677/bid_trade.txt")?;
    let depth_file = File::open("data/capture/btcusdt/1702798595534677/depth.txt")?;

    // Create buffered readers for each file
    let mut ask_trade_reader = io::BufReader::new(ask_trade_file).lines();
    let mut bid_trade_reader = io::BufReader::new(bid_trade_file).lines();
    let mut depth_reader = io::BufReader::new(depth_file).lines();

    let mut next_ask_trade = true;
    let mut next_bid_trade = true;

    let mut orderbook = OrderBook::new();
    orderbook.update_with_snapshot(snapshot);

    let mut bid_matcher = TradeMatcher::new(TradeType::Bid);
    let mut ask_matcher = TradeMatcher::new(TradeType::Ask);

    let mut prev_ask_info = PreviousTradeInfo::new();
    let mut prev_bid_info = PreviousTradeInfo::new();

    loop {
        match next_trades(&mut ask_trade_reader, &mut prev_ask_info, &mut ask_matcher, "Ask") {
            Ok(()) => { /* Normal processing */ },
            Err(e) => {
                error!("Error reading trade: {}", e);
                break; // Break out of the loop
            }
        }
        if ask_matcher.number_of_timestamps() == 5 {
            break;
        }
    }
    loop {
        match next_trades(&mut bid_trade_reader, &mut prev_bid_info, &mut bid_matcher, "Bid") {
            Ok(()) => { /* Normal processing */ },
            Err(e) => {
                error!("Error reading trade: {}", e);
                break; // Break out of the loop
            }
        }
        if bid_matcher.number_of_timestamps() == 5 {
            break;
        }
    }
    
    orderbook.print_orderbook(5, "Before Run");
    loop {

        let depth_line = depth_reader.next();

        // Check if all files have reached EOF
        if depth_line.is_none() {
            break;
        }

        if next_ask_trade {
            next_ask_trade = false;
            loop {
                match next_trades(&mut ask_trade_reader, &mut prev_ask_info, &mut ask_matcher, "Ask") {
                    Ok(()) => { /* Normal processing */ },
                    Err(e) => {
                        error!("Error reading trade: {}", e);
                        break; // Break out of the loop
                    }
                }
                if ask_matcher.number_of_timestamps() == 5 {
                    break;
                }
            }
        }

        if next_bid_trade {
            next_bid_trade = false;
            loop {
                match next_trades(&mut bid_trade_reader, &mut prev_bid_info, &mut bid_matcher, "Bid") {
                    Ok(()) => { /* Normal processing */ },
                    Err(e) => {
                        error!("Error reading trade: {}", e);
                        break; // Break out of the loop
                    }
                }
                if bid_matcher.number_of_timestamps() == 5 {
                    break;
                }
            }
        }

        if let Some(Ok(line)) = depth_line {
            let depth_update: DepthUpdate = serde_json::from_str(&line)?;
            debug!("{:?}", depth_update);
            orderbook.update(depth_update);
            if orderbook.is_best_ask_updated() {
                let event_times = ask_matcher.match_trades(&mut orderbook);
                if event_times.len() != 0 {
                    next_ask_trade = true;
                } 
            }
            if orderbook.is_best_bid_updated() {
                let event_times = bid_matcher.match_trades(&mut orderbook);
                if event_times.len() != 0 {
                    next_bid_trade = true;
                } 
            }
        }
    }

    ask_matcher.purge();
    bid_matcher.purge();
    orderbook.print_orderbook(5, "After Run");

    // After all matching and purging are done
    ask_matcher.clean_trade_results();
    bid_matcher.clean_trade_results();

    // Then print the cleaned trade results
    ask_matcher.print_trade_results();
    bid_matcher.print_trade_results();

    Ok(())
}
