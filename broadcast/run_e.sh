#!/usr/bin/env bash

set -e

# Messages-per-operation is below 20
# Median latency is below 1 second
# Maximum latency is below 2 seconds

cargo build --release
java -jar ~/maelstrom/lib/maelstrom.jar test \
  -w broadcast \
  --bin ~/Projects/gossip-glomers/target/release/broadcast \
  --node-count 25 \
  --time-limit 20 \
  --rate 100 \
  --latency 100
