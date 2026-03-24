FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/raspi-observer /usr/local/bin/raspi-observer

ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8080 \
    ENVIRONMENT=production

EXPOSE 8080

CMD ["raspi-observer"]
