# Stage 1: Build the Rust binary
FROM rust:1.78 as builder

WORKDIR /usr/src/lucius-mcp-worker
COPY . .
RUN cargo build --release

# Stage 2: Create the final image
FROM debian:buster-slim

WORKDIR /root/
COPY --from=builder /usr/src/lucius-mcp-worker/target/release/lucius-mcp-worker .

CMD ["./lucius-mcp-worker"]
