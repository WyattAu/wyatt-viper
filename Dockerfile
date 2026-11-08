# Multi-stage: build the axum server, ship it in a distroless image.
# Build:  docker build -t wyatt-viper .
# Run:    docker run -p 8000:8080 -e PORT=8080 wyatt-viper
FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release -p viper

FROM gcr.io/distroless/cc-debian12
COPY --from=build /app/target/release/viper /viper
ENV PORT=8080
EXPOSE 8080
USER nonroot
CMD ["/viper"]
