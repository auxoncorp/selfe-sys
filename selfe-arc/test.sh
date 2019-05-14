set -e

echo "\nchecking nostd..."
cargo check --no-default-features

echo "\nTesting debug build..."
cargo test

echo "\nTesting release build..."
cargo test --release
