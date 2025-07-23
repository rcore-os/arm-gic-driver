#!/bin/bash
cargo test --target aarch64-unknown-none-softfloat -p test-gicv2 --test test  -- --show-output
