use std::collections::{BTreeSet, VecDeque};

use crate::messages::TradeUpdate;
use crate::orderbook::OrderBook;
use log::{info, debug, warn};

#[derive(Debug)]
pub enum TradeType {
    Bid,
    Ask,
}

pub struct TradeMatcher {
    trade_type: TradeType,
    trade_queue: VecDeque<TradeUpdate>,
    trade_results: BTreeSet<(String, u64, u64)>,
}

impl TradeMatcher {
    pub fn new(trade_type: TradeType) -> Self {
        Self {
            trade_type,
            trade_queue: VecDeque::new(),
            trade_results: BTreeSet::new(),
        }
    }

    pub fn add_trade(&mut self, trade: TradeUpdate) {
        debug!("{:?} - Added Trade ID: {}", self.trade_type, trade.trade_id);
        self.trade_queue.push_back(trade);
    }

    pub fn match_trades(&mut self, orderbook: &mut OrderBook) -> Vec::<u64> {
        let mut indices_to_remove = Vec::new();
        let mut event_times = Vec::<u64>::new();
        let mut trades_to_insert = Vec::new();
    
        debug!("{:?} - *********************************************", self.trade_type);
        debug!("{:?} - Reconciliation attempt on {} trades", self.trade_type, self.trade_queue.len());
    
        for (index, trade) in self.trade_queue.iter().enumerate() {
            debug!("{:?} - Attempting Trade ID: {} - price: {} - quantity: {}", self.trade_type, trade.trade_id, trade.price, trade.quantity);
            let te = match self.trade_type {
                TradeType::Bid => orderbook.match_and_process_trade(trade, TradeType::Bid),
                TradeType::Ask => orderbook.match_and_process_trade(trade, TradeType::Ask),
            };
            
            if te == 1 {
                warn!("{:?} - Dropped Trade ID: {}", self.trade_type, trade.trade_id);
                indices_to_remove.push(index);
                trades_to_insert.push((trade.trade_id.clone(), trade.event_time, te));
            } else if te > 1 {
                info!("{:?} - Matched Trade ID: {}, Event Time: {}", self.trade_type, trade.trade_id, te);
                indices_to_remove.push(index);
                event_times.push(te);
                trades_to_insert.push((trade.trade_id.clone(), trade.event_time, te));
            }
        }
        // Remove items in reverse order
        for index in indices_to_remove.into_iter().rev() {
            self.trade_queue.remove(index);
        }
        // Now that we are no longer borrowing `self.trade_queue`, insert the trades
        for (trade_id, trade_event_time, event_time) in trades_to_insert {
            self.insert_trade_ids(&trade_id,trade_event_time, event_time);
        }        
        event_times
    }
    
    pub fn purge(&mut self) {
        while let Some(trade) = self.trade_queue.pop_front() {
            info!("{:?} - Purged Trade ID: {}", self.trade_type, trade.trade_id);
            self.insert_trade_ids(&trade.trade_id, trade.event_time, 2);
        }
    }

    pub fn number_of_timestamps(&mut self) -> i32 {
        let mut nb_timestamps: i32 = 0;
        let mut old_timestamp: u64 = 0;
        let mut index = 0;
        while index < self.trade_queue.len() {
            let trade = &self.trade_queue[index];
            let timestamp = trade.event_time;
            if old_timestamp != timestamp {
                nb_timestamps += 1;
            }
            old_timestamp = timestamp;
            index += 1;
        }
        return nb_timestamps;
    }

    // Method to clean up trade results
    pub fn clean_trade_results(&mut self) {
        let mut cleaned_results = std::collections::BTreeSet::new();
        let mut trade_id_event_times = std::collections::HashMap::new();
        let mut trade_id_counts = std::collections::HashMap::new();

        // First pass to count occurrences of each trade_id
        for (trade_id, _, _) in &self.trade_results {
            *trade_id_counts.entry(trade_id.clone()).or_insert(0) += 1;
        }

        // Second pass to collect all eligible entries
        for (trade_id, trade_time, event_time) in &self.trade_results {
            let count = trade_id_counts.get(trade_id).cloned().unwrap_or(0);
            if count == 1 || *event_time != 1 && *event_time != 2 {
                // Collect entries of trade_id with various event times
                trade_id_event_times.entry(trade_id.clone()).or_insert_with(Vec::new).push((*trade_time, *event_time));
            }
        }

        // Iterate through the collected entries and retain the desired ones
        for (trade_id, times) in trade_id_event_times {
            if let Some(to_keep) = times.iter().max_by_key(|&(_, event_time)| event_time) {
                cleaned_results.insert((trade_id, to_keep.0, to_keep.1));
            }
        }

        // Replace the old trade_results with the cleaned and ordered results
        self.trade_results = cleaned_results;
    }

    pub fn print_trade_results(&self) {
        let mut results = format!("{:?} - Matching output\n", self.trade_type);
        results.push_str(&format!("\tTrade ID\tTrade Time\tEvent Time\n"));
        for (trade_id, trade_event_time, event_time) in &self.trade_results {
            results.push_str(&format!("\t{}\t{}\t{}\n", trade_id, trade_event_time, event_time));
        }
        // Log the concatenated result
        info!("{}", results.trim_end()); // trim_end to remove the last newline character
    }

    fn insert_trade_ids(&mut self, trade_id: &str, trade_event_time: u64, event_time: u64) {
        let ids: Vec<&str> = trade_id.split('-').collect();
        for id in ids {
            // Use trade_event_time for all the split trade IDs
            self.trade_results.insert((id.to_string(), trade_event_time, event_time));
        }
    }

}
