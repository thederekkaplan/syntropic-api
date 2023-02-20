FROM rust as test
WORKDIR /usr/src/syntropic
COPY . .
CMD ["cargo", "test"]

FROM rust as builder
WORKDIR /usr/src/syntropic
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim as service
COPY --from=builder /usr/src/syntropic/target/release/syntropic-api /usr/local/bin/syntropic-api
CMD ["syntropic-api"]