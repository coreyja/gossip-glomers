#!/usr/bin/env bash

set -e

cargo build
java -jar ~/maelstrom/lib/maelstrom.jar test \
  -w unique-ids \
  --bin ~/Projects/gossip-glomers/target/debug/unique-ids \
  --time-limit 30 \
  --rate 1000 \
  --node-count 3 \
  --availability total \
  --nemesis partition
