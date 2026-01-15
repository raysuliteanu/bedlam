#!/usr/bin/env bash

./maelstrom test -w broadcast --bin "$CARGO_TARGET_DIR"/debug/bedlam --node-count 25 --time-limit 20 --rate 100 --latency 100
