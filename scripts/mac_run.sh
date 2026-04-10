#!/usr/bin/env bash
RUST_LOG=gui=debug,info cargo run -p app --release "$@"
