# Deployment Guide

```bash

# for mac user
DOCKER_DEFAULT_PLATFORM=linux/amd64 ./start_docker.sh

# deploys internet identity
dfx deploy --specified-id rdmx6-jaaaa-aaaaa-aaadq-cai internet_identity --argument '(null)'

dfx deploy backend --argument '(record{
    bitcoin_network = variant { regtest };
    auth = null;
    commission_receiver = null;
})'

```
