# Running

To set up the bot, first install the `sqlx` CLI with:
```bash
cargo install sqlx-cli
```

If there are issues with native tls libraryl, minimal needed features fro install are:
```bash
cargo install sqlx-cli --no-default-features --features sqlite,sqlx-toml
```

## Running Locally

Then, create a `.env` file defining 2 variables `DISCORD_TOKEN` and `DATABASE_URL` like so:
```env
DATABASE_URL="..."
DISCORD_TOKEN="..."
```

Run
```bash
sqlx database create
sqlx migrate run
```
to create a new local competition database.

Use `cargo run` to run the bot.

## Running With Docker

Ensure database is placed at `bot_db/db.sqlite`.

If `.sqlx` needs to be regenerated, us `cargo sqlx prepare`. It only needs to be regenerated after new migrations are written, otherwise `.sqlx` checked into git should be up to date.

Then run:
```bash
docker compose up -d --build
```
