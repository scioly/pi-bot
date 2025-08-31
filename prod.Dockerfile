FROM rust:1.89

ARG version
ENV VERSION=${version}

# Set working directory
WORKDIR /usr/src/app

# Copy all bot code over
COPY . .

# Build and install release binary
RUN cargo install --path . && rm -r /usr/src/app

WORKDIR /

# Run the bot when container is run
CMD ["pi-bot"]
