FROM rust as builder
WORKDIR /usr/src/syntropic
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /usr/src/syntropic/target/release/syntropic-api /usr/local/bin/syntropic-api
CMD ["syntropic-api"]