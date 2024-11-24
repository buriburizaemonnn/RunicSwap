# RunicSwap

### TODO

- Better UTXO management to perform gasless transaction
- UI developement

### Local Deployment

```bash
chmod +x start_docker.sh

# for linux
./start_docker.sh

# for Macos
DOCKER_DEFAULT_PLATFORM=linux/amd64 ./start_docker.sh

dfx start --clean

docker compose exec bitcoind bitcoin-cli getblockhash 1

dfx deploy ord_indexer --specified-id o25oi-jaaaa-aaaal-ajj6a-cai --argument '("http://[::1]:3000", "<HASH>")'

dfx deploy swap_backend --argument '(record{
auth= null;
commission_receiver_btc= "";
commission_receiver_icp= null;
commission_receiver_principal= null;
bitcoin_network= variant { regtest };
ord_canister= principal "o25oi-jaaaa-aaaal-ajj6a-cai";
})'
```

### Wireframe

- Homepage
  ![Homepage](/docs/swap_page.png)
- Pools
  ![pool](/docs/pools.png)
- Positions
  ![position](/docs/position.png)
