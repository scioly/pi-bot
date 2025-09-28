# Run from root directory
FROM rust:1.89-trixie AS build

# Set working directory
WORKDIR /usr/src/app

COPY . .

# Build release binary
RUN SQLX_OFFLINE=true cargo build --release --package pi-bot

FROM debian:trixie-slim

WORKDIR /

# Install release binary
COPY --from=build /usr/src/app/target/release/pi-bot .
# RUN ls -la / && sleep 10
USER 1000
# Run the bot when container is run
CMD ["./pi-bot"]
