# Shougoutaku

This repository is paired with [this medium post](https://medium.com/@quantitative-modelling-for-fun/a-physicist-view-on-market-microstructure-building-a-rust-program-to-match-a-trade-with-its-3bced14a5ce7)

The main program is a rust application that reconciles trade events with order book events from the Binance websockets. It also has three scripts.

## scripts

- To capture custom data
```
python scripts/capture-depth-trades.py --symbol <SYMBOL> --save-folder data/capture/
```
- to visualize the order book snapshot
```
python scripts/plot-orderbook.py --input-path data/capture/btcusdt/1702798595534677/snapshot.txt --output-path tmp/
```
- to visualise the order book changes with the candidates trades within +/- 100ms.
```
python scripts/message-viewer.py
```
(coded using pygame)

## Main program
Reads throught a snapshot, an order book event capture and a trade event capture to produce a suggested reconciliation based on exact match of quantity and price, assuming that there is maximum 100ms lag between the trade event and the order book event.
```
RUST_LOG=info cargo run --bin shougoutaku -- --snapshot data/capture/btcusdt/1702798595534677/snapshot.txt --depth data/capture/btcusdt/1702798595534677/depth.txt --ask_trade data/capture/btcusdt/1702798595534677/ask_trade.txt --bid_trade data/capture/btcusdt/1702798595534677/bid_trade.txt
```
