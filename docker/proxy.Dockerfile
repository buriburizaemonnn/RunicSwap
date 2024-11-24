FROM rust:1.74

WORKDIR /usr/src/btc-rpc-proxy
COPY . .

RUN cargo install --path btc-rpc-proxy/

CMD ["/usr/local/cargo/bin/btc-rpc-proxy", "--forward", "http://bitcoind:18443", "--port", "3000"]
