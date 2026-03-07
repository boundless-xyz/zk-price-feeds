# ZK Oracle -- multi-stage Docker build
# Builds the keeper, boundless, and wizard binaries.

FROM rust:1.89-bookworm AS builder

# Install foundry for contract deployment (optional, used by wizard)
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY host/ host/
COPY methods/ methods/
COPY contracts/ contracts/

# Build all binaries
RUN cargo build --release --bin keeper --bin boundless --bin wizard

# ─── Runtime image ────────────────────────────────────────────────────

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/keeper /usr/local/bin/keeper
COPY --from=builder /build/target/release/boundless /usr/local/bin/boundless
COPY --from=builder /build/target/release/wizard /usr/local/bin/wizard

# Default to the keeper binary
ENTRYPOINT ["keeper"]
