#!/bin/bash

HASH=$(docker compose exec bitcoind bitcoin-cli getblockhash 1)

dfx deploy ord_indexer --specified-id o25oi-jaaaa-aaaal-ajj6a-cai --argument '("http://[::1]:3000", "'"$HASH"'")'

dfx deploy --specified-id rdmx6-jaaaa-aaaaa-aaadq-cai internet_identity --argument '(null)'

dfx deploy backend --argument '(record{
    bitcoin_network = variant { regtest };
    auth = null;
    commission_receiver = null;
})'

dfx deploy frontend
