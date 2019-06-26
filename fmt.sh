#!/usr/bin/env bash
cargo fmt

(
    cd selfe-config
    cargo fmt
)

(
    cd example_application
    cargo fmt
)

(
    cd example_application/sel4-start
    cargo fmt
)

(
    cd selfe-arc
    cargo fmt
)

(
    cd selfe-config
    cargo fmt
)

