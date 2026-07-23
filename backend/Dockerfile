# syntax=docker/dockerfile:1

# --- Plan the dependency graph so it caches independently of source churn ---
FROM rust:1-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Build: compile dependencies (cached), then the crate ---
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin magpie

# --- Runtime: distroless, static-ish binary (SQLite is bundled) ---
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/magpie /usr/local/bin/magpie
ENV MAGPIE_BIND=0.0.0.0:8787 \
    MAGPIE_DATA_DIR=/data
EXPOSE 8787
VOLUME ["/data"]
ENTRYPOINT ["/usr/local/bin/magpie"]
