#!/usr/bin/env bash

./maelstrom test -w echo --bin $CARGO_TARGET_DIR/debug/bedlam --node-count 1 --time-limit 10
