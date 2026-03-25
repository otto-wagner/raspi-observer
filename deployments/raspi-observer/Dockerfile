FROM --platform=$BUILDPLATFORM rust:1-bookworm AS builder

ARG BUILDPLATFORM
ARG TARGETPLATFORM

# Map Docker platform to Rust target triple
RUN case "$TARGETPLATFORM" in \
      "linux/arm64") echo "aarch64-unknown-linux-gnu" > /rust_target ;; \
      "linux/amd64") echo "x86_64-unknown-linux-gnu" > /rust_target ;; \
      *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac

# Install cross-compilation toolchain for arm64
RUN apt-get update && apt-get install -y \
      gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Add the Rust target toolchain
RUN rustup target add $(cat /rust_target)

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Configure linker for cross-compilation and build
RUN mkdir -p .cargo && \
    echo "[target.aarch64-unknown-linux-gnu]" >> .cargo/config.toml && \
    echo 'linker = "aarch64-linux-gnu-gcc"' >> .cargo/config.toml && \
    cargo build --release --locked --target "$(cat /rust_target)"

# Copy binary to a fixed location regardless of target
RUN cp /app/target/$(cat /rust_target)/release/raspi-observer /app/raspi-observer

FROM debian:bookworm-slim AS runtime

COPY --from=builder /app/raspi-observer /usr/local/bin/raspi-observer

ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8080 \
    ENVIRONMENT=production

EXPOSE 8080

CMD ["raspi-observer"]
