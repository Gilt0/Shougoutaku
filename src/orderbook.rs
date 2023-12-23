use rust_decimal::Decimal;
use std::collections::BTreeMap;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::messages::{DepthUpdate, SnapShotUpdate, TradeUpdate};
use crate::trade_matcher::{TradeType};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LevelDelta {
    price: Decimal,
    volume: Decimal,
    event_time: u64,
}

impl LevelDelta {
    pub fn new(price: Decimal, volume: Decimal, event_time: u64) -> Self {
        Self {
            price: price,
            volume: volume,
            event_time: event_time,
        }
    }

}

// Order Book struct
pub struct OrderBook {
    last_update_id: u64,
    first_update_id_in_event: u64,
    final_update_id_in_event: u64,
    bids: BTreeMap<Decimal, Decimal>,
    asks: BTreeMap<Decimal, Decimal>,
    best_bid_updated: bool,
    best_ask_updated: bool,
    best_bid_deltas: Vec::<LevelDelta>,
    best_ask_deltas: Vec::<LevelDelta>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            last_update_id: 0,
            first_update_id_in_event: 0,
            final_update_id_in_event: 0,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            best_bid_updated: false,
            best_ask_updated: false,
            best_bid_deltas: Vec::<LevelDelta>::new(),
            best_ask_deltas: Vec::<LevelDelta>::new(),
        }
    }

    // Getter methods for best bid and ask updated flags
    pub fn is_best_bid_updated(&self) -> bool {
        debug!("Best bid updated");
        self.best_bid_updated
    }

    pub fn is_best_ask_updated(&self) -> bool {
        debug!("Best ask updated");
        self.best_ask_updated
    }

    pub fn update(&mut self, update: DepthUpdate) {
        // Skip if orderbok has not been loaded
        if self.last_update_id == 0 { return; }
        // Skip if final_update_id_in_event <= last_update_id
        if update.final_update_id_in_event <= self.last_update_id { return; }
        if self.first_update_id_in_event == 0 && update.first_update_id_in_event > self.last_update_id + 1 { 
            warn!("Snapshot is a little too old - first_update_id_in_event = {:?}  last_update_id = {:?}", update.first_update_id_in_event, self.last_update_id);
        }
        if self.first_update_id_in_event != 0 && update.first_update_id_in_event != self.final_update_id_in_event + 1 { 
            warn!("Update sequence from websocket is mechakucha - update.first_update_id_in_event = {:?}  self.final_update_id_in_event = {:?}", update.first_update_id_in_event, self.final_update_id_in_event);
        }
        self.first_update_id_in_event = update.first_update_id_in_event;
        self.final_update_id_in_event = update.final_update_id_in_event;
        // Store the current best bid and ask prices
        let current_best_bid = self.bids.keys().next_back().cloned();
        let current_best_ask = self.asks.keys().next().cloned();
        // Reset flags
        self.best_bid_updated = false;
        self.best_ask_updated = false;
        // Reset LevelDeltas before each update
        self.best_bid_deltas.clear();
        self.best_ask_deltas.clear();
        // Update bids
        let mut add_next_bid_level_delta: bool = false;
        for (price_level, quantity) in update.bids_to_update {
            debug!("{:?} - bid = {} quantity = {}", "Bid", price_level, quantity);
            // Check if the best bid price is updated
            if Some(&price_level) == current_best_bid.as_ref() || add_next_bid_level_delta {
                if let Some(current_volume) = self.bids.get(&price_level) {
                    self.best_bid_updated = true;
                    let volume_delta = *current_volume - quantity;
                    if quantity.is_zero() {
                        add_next_bid_level_delta = true;
                    } else {
                        add_next_bid_level_delta = false;
                    }
                    let level_delta = LevelDelta::new(price_level, volume_delta, update.event_time);
                    debug!("{:?} - Best Delta {}", "Bid", serde_json::to_string(&level_delta).unwrap());
                    self.best_bid_deltas.push(level_delta);
                }
            }
            if quantity.is_zero() { self.bids.remove(&price_level); }
            else { self.bids.insert(price_level, quantity); }
        }

        // Update asks
        let mut add_next_ask_level_delta: bool = false;
        for (price_level, quantity) in update.asks_to_update {
            debug!("{:?} - ask = {} quantity = {}", "Ask", price_level, quantity);
           // Check if the best ask price is updated
            if Some(&price_level) == current_best_ask.as_ref() || add_next_ask_level_delta {
                if let Some(current_volume) = self.asks.get(&price_level) {
                    self.best_ask_updated = true;
                    let volume_delta = *current_volume - quantity;
                    if quantity.is_zero() {
                        add_next_ask_level_delta = true;
                    } else {
                        add_next_ask_level_delta = false;
                    }
                    let level_delta = LevelDelta::new(price_level, volume_delta, update.event_time);
                    debug!("{:?} - Best Delta {}", "Bid", serde_json::to_string(&level_delta).unwrap());
                    self.best_ask_deltas.push(level_delta);
                }
            }
            if quantity.is_zero() { self.asks.remove(&price_level); } 
            else { self.asks.insert(price_level, quantity); }
        }
    }

    pub fn update_with_snapshot(&mut self, update: SnapShotUpdate) {
        self.last_update_id = update.last_update_id;
        debug!("snapshot update -- update.last_update_id = {}", update.last_update_id);
        self.print_orderbook(10, "State of the order book before snapshot update");
        // Clear existing bids and asks
        self.bids.clear();
        self.asks.clear();
        // Update bids
        for (price, quantity) in update.bids {
            self.bids.insert(price, quantity);
        }
        // Update asks
        for (price, quantity) in update.asks {
            self.asks.insert(price, quantity);
        }
        self.print_orderbook(10, "State of the order book after snapshot update");
    }

    pub fn match_and_process_trade(&mut self, trade: &TradeUpdate, trade_type: TradeType) -> u64 {
        let level_deltas = match trade_type {
            TradeType::Bid => &mut self.best_bid_deltas,
            TradeType::Ask => &mut self.best_ask_deltas,
        };
        for level_delta in level_deltas.iter_mut() {
            debug!("{:?} - Level Delta {}", trade_type, serde_json::to_string(&level_delta).unwrap());
            debug!("{:?} - Trade {}", trade_type, serde_json::to_string(&trade).unwrap());
            if level_delta.volume < Decimal::new(0, 0) { continue; }
            let trade_event_time = trade.event_time;
            debug!("{:?} - trade_id = {} level_delta.event_time = {} trade_event_time + 100 = {}", trade_type, trade.trade_id, level_delta.event_time, trade_event_time + 100);
            if level_delta.event_time > (trade_event_time + 100) {
                return 1;
            }
            if trade.price == level_delta.price && trade.quantity == level_delta.volume {
                // Here, you'd perform the matching logic and return the trade_id if matched
                debug!("{:?} - Matched event:{} with trade {} - event.volume = {}", trade_type, serde_json::to_string(&level_delta).unwrap(), serde_json::to_string(&trade).unwrap(), level_delta.volume);
                level_delta.volume -= trade.quantity;
                return level_delta.event_time;
            }
        }
        0
    }

    pub fn print_orderbook(&self, n: usize, title: &str) {
        let bid_len = self.bids.len();
        let ask_len = self.asks.len();
        let n = n.min(bid_len.min(ask_len));
        let mut log_output = Vec::new();
        log_output.push(format!("{}\n\t\tBID\t\t\t\t\tASK", title));
        let mut bid_iter = self.bids.iter().rev(); // BTreeMap is sorted in ascending order
        let mut ask_iter = self.asks.iter();

        for _ in 0..n {
            let bid = bid_iter.next()
                .map(|(price, volume)| format!("{} -- {:.15}", price, volume))
                .unwrap_or_default();
            let ask = ask_iter.next()
                .map(|(price, volume)| format!("{} -- {:.15}", price, volume))
                .unwrap_or_default();
                log_output.push(format!("{:<15}\t|\t{}", bid, ask));
        }
        // Log all at once at the end
        let final_log = log_output.join("\n");
        info!("{}", final_log);
    }
}
