# DCA Bitcoin Bot

A Rust-based Telegram bot for automated and manual Bitcoin Dollar Cost Averaging (DCA) on the [Bitkub](https://www.bitkub.com/) exchange.

## Features

-   **Manual Buy**: Buy BTC instantly or place limit orders.
-   **Smart Limit Orders**: Automatically fetches the lowest ask price to place limit orders.
-   **Automated DCA**: Schedule recurring buys using Cron syntax (e.g., daily, weekly).
-   **Portfolio Tracking**: Check wallet balances (THB/BTC) and order history.
-   **Transaction Logging**: all trades are logged to a PostgreSQL database.
-   **Secure**: Restricts access to a specific Telegram User ID.

## Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable)
-   PostgreSQL Database
-   Bitkub API Key & Secret
-   Telegram Bot Token
-   [Just](https://github.com/casey/just) (Optional, for running tasks)

## Installation & Setup

1.  **Clone the repository**
    ```bash
    git clone <repository_url>
    cd dca_btc
    ```

2.  **Configuration**
    Copy the example environment file and update it with your credentials:
    ```bash
    cp .env.example .env
    ```

    **`.env` Configuration:**
    ```ini
    # Database Connection
    DATABASE_URL=postgres://user:password@localhost:5432/dca_btc

    # Telegram Bot Settings
    TELOXIDE_TOKEN=your_telegram_bot_token
    AUTHORIZED_USER_ID=123456789  # Get your ID from @userinfobot

    # Bitkub API Credentials
    API_KEY=your_bitkub_api_key
    SECRET_KEY=your_bitkub_api_secret

    # DCA Scheduler Settings
    SCHEDULE_ENABLED=true       # Set to true to enable auto-buy
    SCHEDULE_AMOUNT=100         # Amount in THB per buy
    SCHEDULE_CRON="0 0 8 * * *" # Cron expression (e.g., Every day at 8:00 AM)
    ```

3.  **Database Setup**
    Ensure PostgreSQL is running. The application handles migrations automatically on startup.
    If you have `just` installed, you can look at the `justfile` for helper commands, but generally:
    ```bash
    # Create database manually if needed
    createdb dca_btc
    ```

4.  **Build & Run**
    ```bash
    cargo run --release
    ```

## Usage

### Telegram Commands

Once the bot is running, you can interact with it via Telegram:

| Command | Description | Example |
| :--- | :--- | :--- |
| `/help` | Show available commands | `/help` |
| `/buylimit <amount>` | Buy BTC with a limit order at the current best price | `/buylimit 500` |
| `/balance` | Show current THB and BTC wallet balances | `/balance` |
| `/history <limit>` | Show recent order history | `/history 5` |
| `/orderinfo` | Show details of the last trade | `/orderinfo` |
| `/status` | Check if the bot is online | `/status` |

### Scheduling (DCA)

To enable automated DCA, set `SCHEDULE_ENABLED=true` in your `.env` file.
The format for `SCHEDULE_CRON` is:
```
sec  min   hour   day of month   month   day of week   year
 *    *     *         *            *         *          *
```
*Example: `0 0 8 * * *` = Every day at 08:00 AM (Asia/Bangkok time).*

## Deployment

This project simplifies deployment using `just`. The deployment process automatically syncs the source code, builds the project on the remote server (to ensure compatibility), and restarts the systemd service.

1.  **Deploy to Server**
    Run the following command, replacing `user@host` with your server's SSH connection string:
    ```bash
    just deploy user@192.168.1.100
    ```
    
    This command will:
    - Sync source code (excluding `target/` and `.git/`) to `/opt/dca_btc/`.
    - Update the systemd service file.
    - Build the project in release mode on the server.
    - Restart the `dca_btc` service.

## License

[MIT](LICENSE)
