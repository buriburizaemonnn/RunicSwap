# RunicSwap

### Deployment Guide

```bash
chmod +x gen_candid.sh
./gen_candid.sh

# for linux
./start_docker.sh

# for Macos
DOCKER_DEFAULT_PLATFORM=linux/amd64 ./start_docker.sh

docker compose exec ord ord --regtest wallet create

# returns a address
docker compose exec ord ord --regtest wallet receive

# funds the address
docker compose exec bitcoind bitcoin-cli generatetoaddress 1 <ADDRESS>

docker compose exec bitcoind bitcoin-cli generatetoaddress 101 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

docker compose exec ord ord --regtest wallet batch --batch /data/etch.yaml --fee-rate 1

docker compose exec bitcoind bitcoin-cli generatetoaddress 6 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

docker compose exec bitcoind bitcoin-cli generatetoaddress 6 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

docker compose exec bitcoind bitcoin-cli generatetoaddress 6 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

# run this command in different window
dfx start --clean

# returns hash
docker compose exec bitcoind bitcoin-cli getblockhash 1

dfx deploy ord_indexer --specified-id o25oi-jaaaa-aaaal-ajj6a-cai --argument '("http://[::1]:3000", "<HASH>")'

dfx deploy swap_backend --argument '(variant { regtest })'

# returns addresses
dfx canister call swap_backend get_deposit_addresses
```
