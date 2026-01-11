#!/usr/bin/env bash

#./maelstrom test -w broadcast --bin "$CARGO_TARGET_DIR"/debug/bedlam --node-count "$@" --time-limit 20 --rate 10
./maelstrom test -w broadcast --bin "$CARGO_TARGET_DIR"/debug/bedlam --node-count 5 --time-limit 20 --rate 10 --nemesis partition
