#!/usr/bin/env bash

set -e

# Messages-per-operation is below 30
# Median latency is below 400ms
# Maximum latency is below 600ms

cargo build --release
java -jar ~/maelstrom/lib/maelstrom.jar test \
  -w broadcast \
  --bin ~/Projects/gossip-glomers/target/release/broadcast \
  --node-count 25 \
  --time-limit 20 \
  --rate 100 \
  --latency 100
