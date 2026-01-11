#!/usr/bin/env bash

./maelstrom test -w unique-ids --bin $CARGO_TARGET_DIR/debug/bedlam --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition
