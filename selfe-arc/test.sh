set -e
echo "\nselfe-arc checks and tests..."

echo "\nChecking nostd..."
cargo check --no-default-features

echo "\nConfirming stable build nostd..."
cargo +stable build --no-default-features

echo "\nConfirming nightly build nostd..."
cargo +stable build --no-default-features

echo "\nTesting debug build..."
cargo test

echo "\nTesting release build..."
cargo test --release
