# SpecSavers Free Appointment Checker

A Rust script that monitors SpecSavers for available appointment slots and sends notifications to Discord.

## Features

- Checks SpecSavers GraphQL API for available eye test appointments
- Sends new appointments to Discord via webhook
- Tracks sent appointments to avoid duplicate notifications
- Uses minimal dependencies: minreq (HTTP), nanoserde (JSON), chrono (dates)

## Setup

1. Install Rust if you haven't already:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Set environment variables:
   ```bash
   export DISCORD_WEBHOOK_URL="your_discord_webhook_url_here"
   export GRAPHQL_URL="https://www.specsavers.ie/graphql"  # Optional, defaults to IE
   export STORE_NUMBER="284"  # Optional, defaults to 284
   export DAYS_AHEAD="14"  # Optional, defaults to 14
   ```

3. Build and run:
   ```bash
   cargo build --release
   cargo run
   ```

## Environment Variables

- `DISCORD_WEBHOOK_URL` (required): Your Discord webhook URL
- `GRAPHQL_URL` (optional): The SpecSavers GraphQL endpoint (defaults to "https://www.specsavers.ie/graphql")
- `STORE_NUMBER` (optional): The SpecSavers store number to check (defaults to "284")
- `DAYS_AHEAD` (optional): Number of days ahead to check for appointments (defaults to 14)

## How It Works

1. Calculates the current date and the configured number of days ahead using chrono
2. Queries the SpecSavers API for appointments in that date range
3. Compares results against previously sent appointments (stored in `sent_appointments.json`)
4. Sends new appointments to Discord
5. Updates the tracking file with newly sent appointments

## Running Periodically with systemd

The project includes systemd service and timer files to run the checker automatically every 15 minutes between 9 AM and 5:30 PM, Monday through Saturday.

### Installation

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Edit `specsavers-checker.service` and update the environment variables with your actual values:
   - `DISCORD_WEBHOOK_URL`: Your Discord webhook URL
   - `STORE_NUMBER`: Your SpecSavers store number
   - Other optional variables as needed

3. Copy the systemd files to the systemd directory:
   ```bash
   sudo cp specsavers-checker.service /etc/systemd/system/
   sudo cp specsavers-checker.timer /etc/systemd/system/
   ```

4. Reload systemd to recognize the new files:
   ```bash
   sudo systemctl daemon-reload
   ```

5. Enable and start the timer:
   ```bash
   sudo systemctl enable specsavers-checker.timer
   sudo systemctl start specsavers-checker.timer
   ```

### Managing the Service

Check timer status:
```bash
sudo systemctl status specsavers-checker.timer
```

Check when the timer will run next:
```bash
systemctl list-timers specsavers-checker.timer
```

View service logs:
```bash
sudo journalctl -u specsavers-checker.service -f
```

Manually run the service (for testing):
```bash
sudo systemctl start specsavers-checker.service
```

Stop the timer:
```bash
sudo systemctl stop specsavers-checker.timer
```

Disable the timer (prevent it from starting on boot):
```bash
sudo systemctl disable specsavers-checker.timer
```

### Alternative: Cron

Alternatively, you can use a cron job:
```bash
# Check every 15 minutes during business hours
*/15 9-17 * * 1-6 cd /home/pplaczek/specsavers-free-appointment-checker && /home/pplaczek/specsavers-free-appointment-checker/target/release/specsavers-checker
```
