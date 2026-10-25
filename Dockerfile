# Multi-stage build:
#   chef     - toolchain + cargo-chef
#   planner  - computes a dependency "recipe" from Cargo.toml/Cargo.lock
#   builder  - builds dependencies (cached layer), then the app
#   runtime  - distroless, non-root, just the binary (~30 MB)
#
# Build for Graviton (cheaper on ECS/Fargate/EC2):
#   docker buildx build --platform linux/arm64 -t aws_app .

ARG RUST_VERSION=1.99

FROM rust:${RUST_VERSION}-slim-bookworm AS chef
# utoipa-swagger-ui's build script downloads the Swagger UI bundle with curl.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Dependencies only: this layer is reused until Cargo.toml/Cargo.lock change.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Migrations are embedded into the binary by `sqlx::migrate!`.
RUN cargo build --release --locked --bin aws_app

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/aws_app /app/aws_app

# Bind on all interfaces so the container port is reachable. DATABASE_URL and
# JWT_SECRET must be supplied at runtime (e.g. ECS task secrets from Secrets
# Manager / SSM Parameter Store); never bake them into the image.
ENV APP_HOST=0.0.0.0 \
    APP_PORT=8080 \
    RUST_LOG=info

EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/aws_app"]
