#!/bin/bash
cargo test --target aarch64-unknown-none-softfloat -p test-base --test test --features qemu -- --show-output
