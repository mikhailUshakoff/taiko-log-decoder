FROM rust:1.88 AS builder

# Set the working directory inside the container
WORKDIR /usr/src/build

# Install the toolchain components
RUN rustup show

# Copy the entire project into the container
COPY . .

# Build the project in release mode
RUN cargo build --release

# Use ubuntu as the base image
FROM ubuntu:latest

# Install CA certificates and any runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /usr/src/build/target/release/taiko-log-decoder /usr/local/bin/taiko-log-decoder

# Set the environment variable
ENV RUST_LOG=info

# Run the binary
CMD ["taiko-log-decoder"]