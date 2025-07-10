#!/bin/bash
cargo test --target aarch64-unknown-none-softfloat -p test-some-rt --test test --features "qemu,hv" -- --show-output
