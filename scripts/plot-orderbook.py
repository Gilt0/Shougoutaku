import json
import matplotlib.pyplot as plt
import numpy as np
import argparse

# Set up argparse to accept file path as a command-line argument
parser = argparse.ArgumentParser(description="Plot cumulative volume data for bids and asks.")
parser.add_argument('--input-path', type=str, help='The path to the data file.', required=True)
parser.add_argument('--output-path', type=str, help='The path to image folder.', required=True)
args = parser.parse_args()

# Read the data from the file
with open(args.input_path, 'r') as file:
    data_string = file.read()

# Parse the JSON data
data = json.loads(data_string)

# Extract bids and asks
bids = data['bids']
asks = data['asks']

# Convert string to float and sort bids in decreasing and asks in increasing order of price
bids = sorted([[float(price), float(volume)] for price, volume in bids], reverse=True)
asks = sorted([[float(price), float(volume)] for price, volume in asks])

# Calculate cumulative volume
bid_volumes = np.array([volume for price, volume in bids])
ask_volumes = np.array([volume for price, volume in asks])
cumulative_bid_volumes = np.cumsum([volume for price, volume in bids])
cumulative_ask_volumes = np.cumsum([volume for price, volume in asks])

# Extracting prices and volumes for plotting
bid_prices = [price for price, volume in bids]
ask_prices = [price for price, volume in asks]

# Plotting
plt.figure(figsize=(10, 6))

# Plot bids
plt.plot(bid_prices, cumulative_bid_volumes, label="Bids", color="green")

# Plot asks
plt.plot(ask_prices, cumulative_ask_volumes, label="Asks", color="red")

# Adding labels and title
plt.title('Cumulative Volume by Price')
plt.xlabel('Price')
plt.ylabel('Cumulative Volume')
plt.ylim([0, 2*min(max(cumulative_bid_volumes), max(cumulative_ask_volumes))])
plt.legend()

# Show the plot
plt.grid(True)
plt.savefig(f'{args.output_path}/ob_cum.png')
