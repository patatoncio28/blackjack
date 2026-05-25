# Stage 1: Planner
FROM lukemathwalker/cargo-chef:latest-rust-1.91-slim-bookworm AS planner
WORKDIR /app
COPY core /app/core
COPY games/blackjack /app/games/blackjack
WORKDIR /app/games/blackjack
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cacher
FROM lukemathwalker/cargo-chef:latest-rust-1.91-slim-bookworm AS cacher
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/core /app/core
COPY --from=planner /app/games/blackjack /app/games/blackjack
COPY --from=planner /app/games/blackjack/recipe.json recipe.json
WORKDIR /app/games/blackjack
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Builder
FROM lukemathwalker/cargo-chef:latest-rust-1.91-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY core /app/core
COPY games/blackjack /app/games/blackjack
COPY --from=cacher /app/games/blackjack/recipe.json /app/games/blackjack/recipe.json
COPY --from=cacher /app/games/blackjack/target /app/games/blackjack/target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
WORKDIR /app/games/blackjack
RUN cargo build --release

# Stage 4: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/games/blackjack/target/release/blackjack /app/blackjack
COPY games/blackjack/ui /app/ui
EXPOSE 3000
CMD ["/app/blackjack"]
