# RunicSwap

<hr>

RunicSwap is a decentralized exchange (DEX) for trading Runestone, built on the ICP blockchain. Leveraging ICP's canister technology, RunicSwap enables direct Bitcoin blockchain transactions, offering decentralized liquidity pools for trading pairs like Bitcoin, ICP, and ckBTC.

With a Uniswap v2-inspired AMM model, RunicSwap simplifies token listing for new projects, unifies fragmented markets, and eliminates reliance on centralized exchanges. Our goal is to provide a seamless, secure, and efficient trading platform for the Runestone ecosystem while enabling future expansion into cross-chain trading with Ethereum, Solana, and more.

Explore, trade, and contribute to the future of decentralized Runestone trading!

### TODO

- Frontend Development: Design and implement the user interface.
- ICP Support: Integrate ICP trading pairs into the exchange.
- ckBTC Support: Enable seamless trading for ckBTC.

### [Pitch + Demo](https://youtu.be/gpu5gsdt7sA)

### Deployment Guide

```bash

# for mac user
DOCKER_DEFAULT_PLATFORM=linux/amd64 ./start_docker.sh

# for linux
./start_docker.sh

docker compose exec ord ord --regtest wallet create

# returns a address
docker compose exec ord ord --regtest wallet receive

# funds the address
docker compose exec bitcoind bitcoin-cli generatetoaddress 1 <ADDRESS>

docker compose exec bitcoind bitcoin-cli generatetoaddress 101 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

docker compose exec ord ord --regtest wallet batch --batch /data/etch.yaml --fee-rate 1

docker compose exec bitcoind bitcoin-cli generatetoaddress 6 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

docker compose exec bitcoind bitcoin-cli generatetoaddress 6 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395

dfx start --clean

chmod +x initial_setup.sh
./initial_setup.sh

# returns addresses
dfx canister call swap_backend get_deposit_addresses

docker compose exec bitcoind bitcoin-cli generatetoaddress 1 <ADDRESS>

docker compose exec ord ord --regtest wallet send <BITCOIN-ADDRESS> 100000:THESE•WILL•BE•WORTHLESS --fee-rate 1

docker compose exec bitcoind bitcoin-cli generatetoaddress 3 bcrt1prkk7t5x47shuz3m6t5k6kul7jd0xgek3aasgcvl043w9wlvny0uq48w395
```
