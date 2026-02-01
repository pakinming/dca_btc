# Builder Stage
FROM rust:1.93.0-alpine AS builder
WORKDIR /app
# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
COPY . .
# Build release binary (rust:alpine defaults to musl target)
RUN cargo build --release

# Runtime Stage
FROM alpine:latest
WORKDIR /app

# Install timezone data (optional but good for bots) and CA certs
RUN apk add --no-cache tzdata ca-certificates

# Copy binary from builder
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/dca_btc /app/dca_btc

# Set executable permission
RUN chmod +x /app/dca_btc

# Run the binary
CMD ["./dca_btc"]
