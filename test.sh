set -e

(
    cd confignoble
    cargo test
)

(
    cd cotransport
    cargo build
    cargo test
)

(
    cd libsel4-sys-gen
    cargo test
)

(
    cd example
    SEL4_PLATFORM=sabre xargo build --target armv7-unknown-linux-gnueabihf
    SEL4_PLATFORM=pc99 xargo build --target=x86_64-unknown-linux-gnu

    ../cotransport/target/debug/cotransport build --arch x86_64 --platform pc99 --debug
    ../cotransport/target/debug/cotransport build --arch x86_64 --platform pc99 --release
    # ../cotransport/target/debug/cotransport build --arch aarch32 --platform sabre
)

