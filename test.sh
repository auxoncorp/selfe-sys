#!/usr/bin/env bash
set -e
dir=$(pwd)

echo "====================== download toolchains =================================="
toolchains_dir="${dir}/toolchains"
mkdir -p $toolchains_dir

armv7_toolchain="gcc-linaro-7.4.1-2019.02-i686_arm-linux-gnueabihf"
armv7_toolchain_url="https://releases.linaro.org/components/toolchain/binaries/latest-7/arm-linux-gnueabihf/${armv7_toolchain}.tar.xz"
armv7_toolchain_dir="${toolchains_dir}/${armv7_toolchain}"

if [ ! -d "${armv7_toolchain_dir}" ]; then
    (
        cd $toolchains_dir
        curl -LO $armv7_toolchain_url
        tar xf "${armv7_toolchain}.tar.xz"
    )
else
    echo "Using existing armv7 toolchain at ${armv7_toolchain_dir}"
fi

armv8_toolchain="gcc-linaro-7.4.1-2019.02-i686_aarch64-linux-gnu"
armv8_toolchain_url="https://releases.linaro.org/components/toolchain/binaries/latest-7/aarch64-linux-gnu/${armv8_toolchain}.tar.xz"
armv8_toolchain_dir="${toolchains_dir}/${armv8_toolchain}"

if [ ! -d "${armv8_toolchain_dir}" ]; then
    (
        cd $toolchains_dir
        curl -LO $armv8_toolchain_url
        tar xf "${armv8_toolchain}.tar.xz"
    )
else
    echo "Using existing aarch64 toolchain at ${armv8_toolchain_dir}"
fi


echo "====================== run tests =================================="
RUSTFLAGS="-C link-args=-no-pie" cargo +stable build
RUSTFLAGS="-C link-args=-no-pie" cargo +stable test
RUSTFLAGS="-C link-args=-no-pie" cargo +nightly test

(
    cd selfe-config
    cargo +stable test
    cargo +nightly test

    cargo build --bin selfe --features bin
    cargo test --features bin
)

(
    cd selfe-arc
    ./test.sh
)

(
    cd example_application

    (
        export PATH="${armv7_toolchain_dir}/bin:${PATH}"
        echo "++++++++++++ Sabre"
        SEL4_PLATFORM=sabre cargo xbuild --target armv7-unknown-linux-gnueabihf

        echo "++++++++++++ TX1"
        SEL4_PLATFORM=tx1 cargo xbuild --target aarch64-unknown-linux-gnu
    )

    SEL4_PLATFORM=pc99 cargo xbuild --target=x86_64-unknown-linux-gnu

    (
        export PATH="${armv8_toolchain_dir}/bin:${PATH}"
        echo "++++++++++++ virt"
        SEL4_PLATFORM=virt cargo xbuild --target aarch64-unknown-linux-gnu
    )

    ../selfe-config/target/debug/selfe build --sel4_arch x86_64 --platform pc99 --debug
    ../selfe-config/target/debug/selfe build --sel4_arch x86_64 --platform pc99 --release

    (
        export PATH="${armv7_toolchain_dir}/bin:${PATH}"
        echo "++++++++++++ Sabre E2E"
        ../selfe-config/target/debug/selfe build --sel4_arch aarch32 --platform sabre
    )


    (
        export PATH="${armv8_toolchain_dir}/bin:${PATH}"
        echo "++++++++++++ Virt E2E"
        ../selfe-config/target/debug/selfe build --sel4_arch aarch64 --platform virt
    )
)

