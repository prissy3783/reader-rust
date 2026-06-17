# Dockerfile for aarch64 - using pre-built binary
# Build binary first: cargo build --release --target aarch64-unknown-linux-musl
# Build frontend first: cd frontend && npm install && npm run build

FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata curl sqlite-libs

WORKDIR /app

COPY target/aarch64-unknown-linux-musl/release/reader-rust /app/reader-rust
COPY frontend/dist /app/web/dist

RUN mkdir -p /app/storage/assets

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV DATABASE_URL=sqlite:/app/storage/reader.db?mode=rwc
ENV STORAGE_DIR=/app/storage
ENV ASSETS_DIR=/app/storage/assets
ENV WEB_ROOT=/app/web/dist
ENV LOG_LEVEL=info

EXPOSE 8080
VOLUME ["/app/storage"]

CMD ["./reader-rust"]
