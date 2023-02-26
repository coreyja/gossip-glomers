#!/usr/bin/env bash

set -e

cargo build
java -jar ~/maelstrom/lib/maelstrom.jar test -w echo --bin ~/Projects/gossip-glomers/target/debug/echo --node-count 1 --time-limit 10
