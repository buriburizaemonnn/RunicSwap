#!/bin/bash

# Test direct Bitcoin RPC (IPv4)
echo "Testing direct Bitcoin RPC via IPv4..."
curl -X POST \
  --user icp:test \
  http://127.0.0.1:18443 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"1.0","id":"btc0","method":"getbestblockhash","params":[]}' | jq .

# Test direct Bitcoin RPC (IPv6)
echo -e "\nTesting direct Bitcoin RPC via IPv6..."
curl -X POST \
  --user icp:test \
  "http://[::1]:18443" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"1.0","id":"btc0","method":"getbestblockhash","params":[]}' | jq .

# Test via proxy (IPv4)
echo -e "\nTesting via proxy (IPv4)..."
curl -X POST \
  http://127.0.0.1:3000 \
  -H "Content-Type: application/json" \
  -H "X-Idempotency: test1" \
  -d '{"jsonrpc":"1.0","id":"btc0","method":"getbestblockhash","params":[]}' | jq .

# Test via proxy (IPv6)
echo -e "\nTesting via proxy (IPv6)..."
curl -X POST \
  "http://[::1]:3000" \
  -H "Content-Type: application/json" \
  -H "X-Idempotency: test2" \
  -d '{"jsonrpc":"1.0","id":"btc0","method":"getbestblockhash","params":[]}' | jq .

# Optional: Test using the container's IPv6 address (if accessing from outside)
# First, get the container's IPv6 address
IPV6_ADDR=$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.GlobalIPv6Address}}{{end}}' btc-rpc-proxy)
if [ ! -z "$IPV6_ADDR" ]; then
  echo -e "\nTesting via proxy container IPv6 address..."
  curl -X POST \
    "http://[$IPV6_ADDR]:3000" \
    -H "Content-Type: application/json" \
    -H "X-Idempotency: test3" \
    -d '{"jsonrpc":"1.0","id":"btc0","method":"getbestblockhash","params":[]}' | jq .
fi
