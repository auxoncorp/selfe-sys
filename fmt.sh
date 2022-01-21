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
    cd example_application/selfe-start
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

(
    cd selfe-runtime
    cargo fmt
)

