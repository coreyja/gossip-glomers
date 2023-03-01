#!/usr/bin/env bash

set -e

cargo build
java -jar ~/maelstrom/lib/maelstrom.jar test \
  -w broadcast \
  --bin ~/Projects/gossip-glomers/target/debug/broadcast \
  --node-count 5 \
  --time-limit 20 \
  --rate 10 \
  --nemesis partition
