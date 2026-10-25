FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
# wreq compila BoringSSL (btls-sys): precisa de cmake e libclang (bindgen).
# Este estágio também é a imagem de dev: docker build --target chef -t mmt-rust ./backend
RUN apt-get update && apt-get install -y --no-install-recommends cmake clang libclang-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# Mesma base Debian (trixie) do builder: binário compilado com glibc 2.41 não roda no bookworm (2.36).
# Sem libssl: wreq linka BoringSSL estático e o sqlx usa rustls. Migrations vão embutidas no binário (sqlx::migrate!).
FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -u 10001 app
COPY --from=builder /app/target/release/mac-mini-tracker-backend /usr/local/bin/app
USER app
EXPOSE 8080
CMD ["app"]
