set -e

cargo test

(
    cd confignoble
    cargo test

    cargo build --bin cotransport --features bin
    cargo test --features bin
)

(
    cd example_application
    SEL4_PLATFORM=sabre cargo xbuild --target armv7-unknown-linux-gnueabihf
    SEL4_PLATFORM=pc99 cargo xbuild --target=x86_64-unknown-linux-gnu

    ../confignoble/target/debug/cotransport build --sel4_arch x86_64 --platform pc99 --debug
    ../confignoble/target/debug/cotransport build --sel4_arch x86_64 --platform pc99 --release
    ../confignoble/target/debug/cotransport build --sel4_arch aarch32 --platform sabre
)

