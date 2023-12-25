import pygame
import json
import decimal as dc

# Initialize Pygame
pygame.init()

# Window setup
width, height = 800, 600
screen = pygame.display.set_mode((width, height))
pygame.display.set_caption("Orderbook Visualization")

# Colors and Constants
BACKGROUND_COLOR = (255, 255, 255)
BAR_COLOR = (0, 0, 0)
MARKER_COLOR = (255, 0, 0)
ORDER_BOOK_COLOR = (0, 0, 0)
TEXT_COLOR = (0, 0, 0)
BAR_HEIGHT = 40
BAR_Y_POS = height - BAR_HEIGHT - 40  # Leave space for order book display
SPREAD_Y = 450
TRADE_Y = 350
MAX_LEVELS = 10  # Max price levels to display
CURSOR_SPEED = 2  # Adjust this value to change how fast the cursor moves with keyboard arrows


# Initial marker position and state
marker_position = 0
mouse_held_down = False

# Order Book Data
order_books = {}

first_order_book = { "bids": {}, "asks": {} }
# Load and prepare data
with open("data/capture/btcusdt/1702798595534677/snapshot.txt", "r") as file:
    snapshot_data = json.load(file)
    # Initialize order book with snapshot data
    for bid in snapshot_data["bids"]:
        first_order_book["bids"][bid[0]] = dc.Decimal(bid[1])
    for ask in snapshot_data["asks"]:
        first_order_book["asks"][ask[0]] = dc.Decimal(ask[1])

depth_updates = []
with open("data/capture/btcusdt/1702798595534677/depth.txt", "r") as file:
    depth_updates = [json.loads(line) for line in file]


def find_oldest_timestamp(snapshot, depth_updates):
    lastUpdateId = snapshot['lastUpdateId']
    oldest_timestamp = None
    for update in depth_updates:
        if update['U'] <= lastUpdateId <= update['u']:
            oldest_timestamp = update['E']
            break
    return oldest_timestamp

def find_newest_timestamp(depth_updates):
    return depth_updates[-1]['E']


# Initial timestamps
oldest_timestamp = find_oldest_timestamp(snapshot_data, depth_updates)
newest_timestamp = find_newest_timestamp(depth_updates)

# Process depth updates into order books dictionary
order_books[oldest_timestamp] = {"bids": {}, "asks": {}}
for bid in snapshot_data["bids"]:
    order_books[oldest_timestamp]["bids"][bid[0]] = dc.Decimal(bid[1])
for ask in snapshot_data["asks"]:
    order_books[oldest_timestamp]["asks"][ask[0]] = dc.Decimal(ask[1])

# Utility functions
def timestamp_to_position(timestamp, oldest, newest):
    """Convert timestamp to x position on the bar."""
    total_range = newest - oldest
    relative_pos = (timestamp - oldest) / total_range
    return int(relative_pos * width)

def position_to_timestamp(pos, oldest, newest):
    """Convert x position on the bar to a timestamp."""
    relative_pos = pos / width
    return int(relative_pos * (newest - oldest) + oldest)

def update_order_book(order_book, changes, removal_condition=0):
    for price, quantity in changes:
        quantity = dc.Decimal(quantity)
        if quantity == removal_condition:
            if price in order_book:
                del order_book[price]
        else:
            order_book[price] = quantity

# Construct order books for each timestamp
last_timestamp = oldest_timestamp
for update in depth_updates:
    timestamp = update['E']
    order_books[timestamp] = {"bids": order_books[last_timestamp]["bids"].copy(),
                              "asks": order_books[last_timestamp]["asks"].copy()}
    update_order_book(order_books[timestamp]["bids"], update.get('b', []))
    update_order_book(order_books[timestamp]["asks"], update.get('a', []))
    last_timestamp = timestamp


# Function to draw order book with variations
def draw_order_book(screen, order_book, prev_order_book, mid_y):
    font = pygame.font.Font(None, 24)
    spread_y = SPREAD_Y  # Starting Y position

    # Function to format price and volume
    def format_price_volume(price, quantity, prev_quantity):
        formatted_price = f"{dc.Decimal(price)}"
        formatted_volume = f"{quantity}"
        variation = quantity - prev_quantity
        formatted_variation = f"({variation})" if variation else ""
        return f"{formatted_price} | {formatted_volume} {formatted_variation}"

    # Display bids
    sorted_bids = sorted(order_book["bids"].items(), key=lambda x: dc.Decimal(x[0]), reverse=True)[:MAX_LEVELS]
    screen.blit(font.render("Bid", True, ORDER_BOOK_COLOR), (width // 2 - 350, mid_y - spread_y))
    spread_y -= 20
    for price, quantity in sorted_bids:
        text = format_price_volume(price, quantity, prev_order_book["bids"].get(price, 0))
        screen.blit(font.render(text, True, ORDER_BOOK_COLOR), (width // 2 - 350, mid_y - spread_y))
        spread_y -= 20

    # Display asks
    sorted_asks = sorted(order_book["asks"].items(), key=lambda x: dc.Decimal(x[0]))[:MAX_LEVELS]
    spread_y = SPREAD_Y  # Reset spread for asks
    screen.blit(font.render("Ask", True, ORDER_BOOK_COLOR), (width // 2 + 10, mid_y - spread_y))
    spread_y -= 20
    for price, quantity in sorted_asks:
        text = format_price_volume(price, quantity, prev_order_book["asks"].get(price, 0))
        screen.blit(font.render(text, True, ORDER_BOOK_COLOR), (width // 2 + 10, mid_y - spread_y))
        spread_y -= 20

def draw_trades(screen, trades, current_timestamp):
    font = pygame.font.Font(None, 24)
    trade_y = TRADE_Y  # Starting Y position below order book
    # Iterate through trades within the specified time range
    for timestamp, bid_trades in trades["bids"].items():
        if int(current_timestamp) - 100 > int(timestamp) or int(timestamp) > int(current_timestamp) + 100:
            continue
        for bid_trade in bid_trades:
            trade_details = f"({bid_trade['E']}) {bid_trade['p']} | {bid_trade['q']}"
            # print(f'{timestamp} - drawing {trade_details}')
            trade_y += 20  # Move down for each trade
            screen.blit(font.render(trade_details, True, TEXT_COLOR), (width // 2 - 350, trade_y))
    trade_y = TRADE_Y 
    for timestamp, ask_trades in trades["asks"].items():
        if int(current_timestamp) - 100 > int(timestamp) or int(timestamp) > int(current_timestamp) + 100:
            continue
        for ask_trade in ask_trades:
            trade_details = f"({ask_trade['E']}) {ask_trade['p']} | {ask_trade['q']}"
            print(f'{timestamp} - drawing {trade_details}')
            trade_y += 20  # Move down for each trade
            screen.blit(font.render(trade_details, True, TEXT_COLOR), (width // 2 + 10, trade_y))

# Load bid and ask trades
bid_trades = []
with open("data/capture/btcusdt/1702798595534677/bid_trade.txt", "r") as file:
    bid_trades = [json.loads(line) for line in file]

ask_trades = []
with open("data/capture/btcusdt/1702798595534677/ask_trade.txt", "r") as file:
    ask_trades = [json.loads(line) for line in file]

# Organize trades by timestamp
trades_by_timestamp = { "bids": {}, "asks": {} }

for bid_trade in bid_trades:
    timestamp = bid_trade['E']
    if not timestamp in trades_by_timestamp["bids"]:
        trades_by_timestamp["bids"][timestamp] = []
    trades_by_timestamp["bids"][timestamp].append(bid_trade)
for ask_trade in ask_trades:
    timestamp = ask_trade['E']
    if not timestamp in trades_by_timestamp["asks"]:
        trades_by_timestamp["asks"][timestamp] = []
    trades_by_timestamp["asks"][timestamp].append(ask_trade)

print(trades_by_timestamp)

# Initialize state variables for arrow key presses
left_arrow_down = False
right_arrow_down = False

# Main loop
running = True
while running:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            running = False
        elif event.type == pygame.MOUSEBUTTONDOWN:
            if event.pos[1] >= BAR_Y_POS and event.pos[1] <= BAR_Y_POS + BAR_HEIGHT:
                mouse_held_down = True
        elif event.type == pygame.MOUSEBUTTONUP:
            mouse_held_down = False
        elif event.type == pygame.MOUSEMOTION and mouse_held_down:
            mouse_x, _ = event.pos
            if 0 <= mouse_x <= width:
                marker_position = mouse_x
        elif event.type == pygame.KEYDOWN:
            # Update flags based on key press
            if event.key == pygame.K_LEFT:
                left_arrow_down = True
            elif event.key == pygame.K_RIGHT:
                right_arrow_down = True
        elif event.type == pygame.KEYUP:
            # Update flags based on key release
            if event.key == pygame.K_LEFT:
                left_arrow_down = False
            elif event.key == pygame.K_RIGHT:
                right_arrow_down = False

    # Handle continuous movement based on the current state of arrow keys
    if left_arrow_down:
        marker_position = max(0, marker_position - CURSOR_SPEED)
    if right_arrow_down:
        marker_position = min(width, marker_position + CURSOR_SPEED)

    # Clear screen
    screen.fill(BACKGROUND_COLOR)

    # Determine the current and previous timestamps
    current_timestamp = position_to_timestamp(marker_position, depth_updates[0]['E'], newest_timestamp)
    timestamps = sorted(order_books.keys())
    current_index = next(i for i, ts in enumerate(timestamps) if ts >= current_timestamp)
    current_book = order_books[timestamps[current_index]]
    prev_book = order_books[timestamps[max(0, current_index - 1)]]  # Get immediate previous or itself if none


    # Draw timestamp bar and marker
    pygame.draw.rect(screen, BAR_COLOR, (0, BAR_Y_POS, width, BAR_HEIGHT))
    pygame.draw.circle(screen, MARKER_COLOR, (marker_position, BAR_Y_POS + BAR_HEIGHT // 2), 10)

    # Draw order book with variations
    draw_order_book(screen, current_book, prev_book, height - BAR_HEIGHT - 20)

    # Draw Trades
    draw_trades(screen, trades_by_timestamp, current_timestamp)


    # Render and display the closest timestamp
    font = pygame.font.Font(None, 36)
    closest_timestamp = min(depth_updates, key=lambda x: abs(x['E'] - current_timestamp))['E']
    timestamp_text = font.render(f"Timestamp: {closest_timestamp}", True, TEXT_COLOR)
    screen.blit(timestamp_text, (10, 10))

    pygame.display.flip()

pygame.quit()
