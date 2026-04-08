#!/usr/bin/env bash
RUST_LOG=info cargo run -p gui --release "$@"
