set -e

RUSTFLAGS="-C link-args=-no-pie" cargo test

(
    cd selfe-config
    cargo test

    cargo build --bin selfe --features bin
    cargo test --features bin
)

(
    cd selfe-arc
    ./test.sh
)

(
    cd example_application
    SEL4_PLATFORM=sabre cargo xbuild --target armv7-unknown-linux-gnueabihf
    SEL4_PLATFORM=pc99 cargo xbuild --target=x86_64-unknown-linux-gnu

    ../selfe-config/target/debug/selfe build --sel4_arch x86_64 --platform pc99 --debug
    ../selfe-config/target/debug/selfe build --sel4_arch x86_64 --platform pc99 --release
    ../selfe-config/target/debug/selfe build --sel4_arch aarch32 --platform sabre
)

