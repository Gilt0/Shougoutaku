import asyncio
import websockets
import signal
import argparse
import os
import json
import requests
from datetime import datetime

# Parse command line arguments
parser = argparse.ArgumentParser(description="Capture and save Binance WebSocket streams.")
parser.add_argument("-s", "--symbol", help="Trading symbol, e.g., 'BNBBTC'", required=True)
parser.add_argument("-sf", "--save-folder", help="Folder to save the data streams", required=True)
args = parser.parse_args()

lower_symbol = args.symbol.lower()

# Construct WebSocket URLs based on the provided symbol
depth_url = f"wss://stream.binance.com:9443/ws/{lower_symbol}@depth@100ms"
trade_url = f"wss://stream.binance.com:9443/ws/{lower_symbol}@trade"

now = int(1000000*datetime.now().timestamp())

# Function to get a file stream and ensure the directory exists
def get_file_stream(folder, file_prefix):
    folder = f'{folder}/{lower_symbol}/{now}'
    os.makedirs(folder, exist_ok=True)  # Create folder if it doesn't exist
    filename = os.path.join(folder, f"{file_prefix}.txt")
    return open(filename, 'a')

# Function to fetch and save the snapshot
async def fetch_and_save_snapshot(symbol, folder):
    await asyncio.sleep(1)  # Delay for 1 second
    url = f"https://api.binance.com/api/v3/depth?symbol={symbol.upper()}&limit=5000"
    response = requests.get(url)
    if response.status_code == 200:
        snapshot_stream = get_file_stream(folder, "snapshot")
        snapshot_stream.write(response.text)
        snapshot_stream.flush()
        snapshot_stream.close()

async def capture_stream(url, file_prefix, folder, stop):
    async with websockets.connect(url) as websocket:
        while not stop.is_set():
            try:
                message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                if file_prefix == "trade":
                    data = json.loads(message)
                    trade_type = "bid" if data["m"] else "ask"
                    if trade_type == "bid":
                        file_stream_bid = get_file_stream(folder, "bid_trade")
                        file_stream_bid.write(f"{message}\n")
                        file_stream_bid.flush()
                        file_stream_bid.close()
                    else:
                        file_stream_ask = get_file_stream(folder, "ask_trade")
                        file_stream_ask.write(f"{message}\n")
                        file_stream_ask.flush()
                        file_stream_ask.close()
                elif file_prefix == 'depth':
                    file_stream = get_file_stream(folder, file_prefix)
                    file_stream.write(f"{message}\n")
                    file_stream.flush()
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                print(f"Error in {file_prefix} Stream: {e}")

async def main():
    stop = asyncio.Event()

    def signal_handler():
        print("Signal received, stopping...")
        stop.set()

    loop = asyncio.get_event_loop()
    loop.add_signal_handler(signal.SIGINT, signal_handler)
    loop.add_signal_handler(signal.SIGTERM, signal_handler)

    # Start fetching snapshot after starting depth stream
    asyncio.create_task(fetch_and_save_snapshot(args.symbol, args.save_folder))

    depth_task = asyncio.create_task(
        capture_stream(depth_url, "depth", args.save_folder, stop)
    )
    trade_task = asyncio.create_task(
        capture_stream(trade_url, "trade", args.save_folder, stop)
    )

    await asyncio.gather(depth_task, trade_task)

loop = asyncio.get_event_loop()

try:
    loop.run_until_complete(main())
except KeyboardInterrupt:
    print("KeyboardInterrupt caught, shutting down...")
finally:
    loop.close()
    print("Cleanly disconnected from the WebSocket servers.")
