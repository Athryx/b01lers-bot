FROM rust:1.97.1 AS builder

RUN useradd -ms /bin/sh b01lers-bot

USER b01lers-bot
WORKDIR /home/b01lers-bot

COPY Cargo.toml Cargo.lock config.toml config_testing.toml badctf_bingo.png red_x.png ./
COPY src ./src/
COPY .sqlx ./.sqlx/

RUN cargo build --release && mv target/release/b01lers_bot ./b01lers_bot

ENTRYPOINT [ "./b01lers_bot", "--config", "config.toml" ]
