use selfe_config::model::*;
use std::collections::btree_map::BTreeMap;
use std::path::PathBuf;

const EXAMPLE: &str = r#"[build.sabre.debug]
make_root_task = 'cmake debug'
root_task_image = 'debug_image'

[build.sabre.release]
make_root_task = 'cmake release'
root_task_image = 'release_image'
[build.some_arbitrary_platform.debug]
make_root_task = 'cmake debug'
root_task_image = 'debug_image'

[build.some_arbitrary_platform.release]
make_root_task = 'cmake release'
root_task_image = 'release_image'
[sel4.config]
KernelRetypeFanOutLimit = 256

[sel4.config.aarch32]
KernelArmFastMode = true

[sel4.config.aarch64]
KernelArmFastMode = false

[sel4.config.debug]
KernelDebugBuild = true
KernelPrinting = true

[sel4.config.release]
KernelDebugBuild = false
KernelPrinting = false

[sel4.config.sabre]
SomeOtherKey = 'hi'

[sel4.config.some_arbitrary_platform]
SomeOtherKey = 'aloha'

[sel4.kernel]
path = './deps/seL4'

[sel4.tools]
path = './deps/seL4_tools'

[sel4.util_libs]
path = './deps/util_libs'
"#;

#[test]
fn reads_from_external_default_file_okay() {
    let toml_content = include_str!("../src/default_config.toml");
    let f: full::Full = toml_content.parse().expect("could not read toml");
    assert!(!f.sel4.config.shared.is_empty());
}

#[test]
fn full_parse_happy_path() {
    let f: full::Full = EXAMPLE.parse().expect("could not read toml to full");
    assert_eq!(
        SeL4Sources {
            kernel: RepoSource::LocalPath(PathBuf::from("./deps/seL4")),
            tools: RepoSource::LocalPath(PathBuf::from("./deps/seL4_tools")),
            util_libs: RepoSource::LocalPath(PathBuf::from("./deps/util_libs"))
        },
        f.sel4.sources
    );
    assert_eq!(1, f.sel4.config.shared.len());
    let shared_retype = f.sel4.config.shared.get("KernelRetypeFanOutLimit").unwrap();
    assert_eq!(&SingleValue::Integer(256), shared_retype);

    let debug_printing = f.sel4.config.debug.get("KernelPrinting").unwrap();
    assert_eq!(&SingleValue::Boolean(true), debug_printing);
    let release_printing = f.sel4.config.release.get("KernelPrinting").unwrap();
    assert_eq!(&SingleValue::Boolean(false), release_printing);

    let arm32 = f.sel4.config.contextual.get("aarch32").unwrap();
    assert_eq!(1, arm32.len());
    let fast_mode_32 = arm32.get("KernelArmFastMode").unwrap();
    assert_eq!(&SingleValue::Boolean(true), fast_mode_32);

    let arm64 = f.sel4.config.contextual.get("aarch64").unwrap();
    assert_eq!(1, arm64.len());
    let fast_mode_64 = arm64.get("KernelArmFastMode").unwrap();
    assert_eq!(&SingleValue::Boolean(false), fast_mode_64);

    let sabre = f.sel4.config.contextual.get("sabre").unwrap();
    assert_eq!(1, sabre.len());
    let arb_key_sabre = sabre.get("SomeOtherKey").unwrap();
    assert_eq!(&SingleValue::String("hi".to_owned()), arb_key_sabre);

    let some_arbitrary_platform = f
        .sel4
        .config
        .contextual
        .get("some_arbitrary_platform")
        .unwrap();
    assert_eq!(1, some_arbitrary_platform.len());
    let arb_key_some_arbitrary_platform = some_arbitrary_platform.get("SomeOtherKey").unwrap();
    assert_eq!(
        &SingleValue::String("aloha".to_owned()),
        arb_key_some_arbitrary_platform
    );

    let resolved_some_arbitrary_platform_default = contextualized::Contextualized::from_full(
        &f,
        Arch::Arm,
        SeL4Arch::Aarch32,
        true,
        Platform("some_arbitrary_platform".to_owned()),
        None,
    )
    .unwrap();

    let resolved_sabre = contextualized::Contextualized::from_full(
        &f,
        Arch::Arm,
        SeL4Arch::Aarch32,
        true,
        Platform("sabre".to_string()),
        None,
    )
    .unwrap();
    assert_ne!(resolved_some_arbitrary_platform_default, resolved_sabre);
}

#[test]
fn round_trip() {
    assert_round_trip_equivalence(EXAMPLE, true);
}

fn assert_round_trip_equivalence(source: &str, require_exact_reserialization: bool) {
    let f_alpha: full::Full = source.parse().expect("could not read toml");
    let serialized = f_alpha
        .to_toml_string()
        .expect("could not serialize to toml");
    let f_beta: full::Full = serialized.parse().expect("could not read serialized toml");
    assert_eq!(f_alpha, f_beta);
    if require_exact_reserialization {
        assert_eq!(source, serialized);
    }
}

#[test]
fn happy_path_straight_to_contextualized() {
    let f = contextualized::Contextualized::from_str(
        EXAMPLE,
        Arch::Arm,
        SeL4Arch::Aarch32,
        true,
        Platform("sabre".to_owned()),
        None,
    )
    .unwrap();
    assert_eq!(
        SeL4Sources {
            kernel: RepoSource::LocalPath(PathBuf::from("./deps/seL4")),
            tools: RepoSource::LocalPath(PathBuf::from("./deps/seL4_tools")),
            util_libs: RepoSource::LocalPath(PathBuf::from("./deps/util_libs"))
        },
        f.sel4_sources
    );
    assert_eq!(Arch::Arm, f.context.arch);
    assert_eq!(SeL4Arch::Aarch32, f.context.sel4_arch);
    assert_eq!(Platform("sabre".to_owned()), f.context.platform);
    assert_eq!(true, f.context.is_debug);
    println!("{:#?}", f.sel4_config);
    assert_eq!(5, f.sel4_config.len());
    assert_eq!(
        &SingleValue::Integer(256),
        f.sel4_config.get("KernelRetypeFanOutLimit").unwrap()
    );
    assert_eq!(
        &SingleValue::Boolean(true),
        f.sel4_config.get("KernelDebugBuild").unwrap()
    );
    assert_eq!(
        &SingleValue::Boolean(true),
        f.sel4_config.get("KernelPrinting").unwrap()
    );
    assert_eq!(
        &SingleValue::Boolean(true),
        f.sel4_config.get("KernelArmFastMode").unwrap()
    );
    assert_eq!(
        &SingleValue::String("hi".to_owned()),
        f.sel4_config.get("SomeOtherKey").unwrap()
    );
}

const WITH_METADATA: &str = r##"
[sel4]
kernel = { git = "https://github.com/seL4/seL4" , tag = "10.1.1" }
tools = { git = "https://github.com/seL4/seL4_tools" , branch = "10.1.x-compatible" }
util_libs  = { git = "https://github.com/seL4/util_libs" , branch = "10.1.x-compatible" }

[build.pc99]

[build.sabre]

[metadata]
arb-user-data = 1

[metadata.debug]
debug-specific = 2

[metadata.release]
release-specific = 3

[metadata.arm]
arm-specific = 4

[metadata.x86]
x86-specific = 5

[metadata.aarch32]
aarch32-specific = 6

[metadata.aarch64]
aarch64-specific = 7

[metadata.pc99]
in-all-platforms = 8
pc99-specific = 9

[metadata.sabre]
in-all-platforms = 10
sabre-specific = 11
"##;

#[test]
fn metadata_round_trip() {
    assert_round_trip_equivalence(WITH_METADATA, false);
}

#[test]
fn finds_contextualized_metadata() {
    let f: full::Full = WITH_METADATA.parse().expect("could not read toml");

    let arm_aarch32_sabre_debug = contextualized::Contextualized::from_full(
        &f,
        Arch::Arm,
        SeL4Arch::Aarch32,
        true,
        Platform("sabre".to_string()),
        None,
    )
    .expect("Could not contextualize");
    assert_eq!(6, arm_aarch32_sabre_debug.metadata.len());
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "arb-user-data", 1);
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "debug-specific", 2);
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "arm-specific", 4);
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "aarch32-specific", 6);
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "in-all-platforms", 10);
    assert_contains_int(&arm_aarch32_sabre_debug.metadata, "sabre-specific", 11);

    let arm_aarch64_sabre_debug = contextualized::Contextualized::from_full(
        &f,
        Arch::Arm,
        SeL4Arch::Aarch64,
        true,
        Platform("sabre".to_string()),
        None,
    )
    .expect("Could not contextualize");
    assert_eq!(6, arm_aarch64_sabre_debug.metadata.len());
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "arb-user-data", 1);
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "debug-specific", 2);
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "arm-specific", 4);
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "aarch64-specific", 7);
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "in-all-platforms", 10);
    assert_contains_int(&arm_aarch64_sabre_debug.metadata, "sabre-specific", 11);

    let arm_aarch64_sabre_release = contextualized::Contextualized::from_full(
        &f,
        Arch::Arm,
        SeL4Arch::Aarch64,
        false,
        Platform("sabre".to_string()),
        None,
    )
    .expect("Could not contextualize");
    assert_eq!(6, arm_aarch64_sabre_release.metadata.len());
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "arb-user-data", 1);
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "release-specific", 3);
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "arm-specific", 4);
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "aarch64-specific", 7);
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "in-all-platforms", 10);
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "sabre-specific", 11);

    let x86_x86_64_pc99_release = contextualized::Contextualized::from_full(
        &f,
        Arch::X86,
        SeL4Arch::X86_64,
        false,
        Platform("pc99".to_string()),
        None,
    )
    .expect("Could not contextualize");
    assert_eq!(5, x86_x86_64_pc99_release.metadata.len());
    assert_contains_int(&arm_aarch64_sabre_release.metadata, "arb-user-data", 1);
    assert_contains_int(&x86_x86_64_pc99_release.metadata, "release-specific", 3);
    assert_contains_int(&x86_x86_64_pc99_release.metadata, "x86-specific", 5);
    assert_contains_int(&x86_x86_64_pc99_release.metadata, "in-all-platforms", 8);
    assert_contains_int(&x86_x86_64_pc99_release.metadata, "pc99-specific", 9);
}

fn assert_contains_int(map: &BTreeMap<String, SingleValue>, key: &str, val: i64) {
    assert_eq!(
        &SingleValue::Integer(val),
        map.get(key)
            .unwrap_or_else(|| panic!("Did not contain expected key {}", key))
    );
}
