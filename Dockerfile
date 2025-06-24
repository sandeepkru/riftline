# Build stage
FROM rust:1.76-slim as builder
WORKDIR /usr/src/riftline
COPY . .
RUN apt-get update && apt-get install -y musl-tools pkg-config && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo build --release -p server --target x86_64-unknown-linux-musl

# Runtime stage
FROM gcr.io/distroless/static
COPY --from=builder /usr/src/riftline/target/x86_64-unknown-linux-musl/release/server /server
EXPOSE 50051
CMD ["/server"]
