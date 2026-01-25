FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ARG AUTH_FEATURE=auth-local

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo build --release --no-default-features --features ${AUTH_FEATURE}


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p /app/data && chown 1000:1000 /app/data

COPY --from=builder /app/target/release/eko-messenger /usr/local/bin/eko-messenger

USER 1000

ENV PORT=3000
ENV VAPID_KEY_PATH=/app/data/vapid.pem
ENV IP_SOURCE=ConnectInfo

EXPOSE 3000

CMD ["eko-messenger"]
