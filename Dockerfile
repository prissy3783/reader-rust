FROM node:20-alpine AS frontend-builder

WORKDIR /frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.76-alpine AS rust-builder

RUN apk add --no-cache musl-dev openssl-dev pkgconfig sqlite-dev

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --features webdav

FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata curl sqlite-libs

WORKDIR /app

COPY --from=rust-builder /src/target/release/reader-rust /app/reader-rust
COPY --from=frontend-builder /frontend/dist /app/web/dist

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
