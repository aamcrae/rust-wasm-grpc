# syntax=docker/dockerfile:1
FROM rust:1.87-alpine AS builder

WORKDIR /src

#
# Install protobuf compiler and wasm-bindgen
RUN apk update  && apk upgrade
RUN apk add protoc musl-dev
RUN cargo install wasm-bindgen-cli

COPY . .
RUN (cd server; cargo build -r)

RUN rustup target add wasm32-unknown-unknown
RUN (cd client/client; cargo build -r)
RUN wasm-bindgen target/wasm32-unknown-unknown/release/client.wasm --out-dir www --target web
RUN (cd client/worker; cargo build -r)
RUN wasm-bindgen target/wasm32-unknown-unknown/release/worker.wasm --out-dir www --target no-modules

FROM alpine

COPY --from=builder /src/www /www
COPY --from=builder /src/jokes /jokes
COPY --from=builder /src/target/release/server /bin/server

CMD ["/bin/server", "--jokes=/jokes", "--www=/www", "--server=0.0.0.0"]
