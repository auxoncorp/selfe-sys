use confignoble::model::*;
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
    assert!(f.sel4.config.shared_config.len() > 0);
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
    assert_eq!(1, f.sel4.config.shared_config.len());
    let shared_retype = f
        .sel4
        .config
        .shared_config
        .get("KernelRetypeFanOutLimit")
        .unwrap();
    assert_eq!(&SingleValue::Integer(256), shared_retype);

    let debug_printing = f.sel4.config.debug_config.get("KernelPrinting").unwrap();
    assert_eq!(&SingleValue::Boolean(true), debug_printing);
    let release_printing = f.sel4.config.release_config.get("KernelPrinting").unwrap();
    assert_eq!(&SingleValue::Boolean(false), release_printing);

    let arm32 = f.sel4.config.contextual_config.get("aarch32").unwrap();
    assert_eq!(1, arm32.len());
    let fast_mode_32 = arm32.get("KernelArmFastMode").unwrap();
    assert_eq!(&SingleValue::Boolean(true), fast_mode_32);

    let arm64 = f.sel4.config.contextual_config.get("aarch64").unwrap();
    assert_eq!(1, arm64.len());
    let fast_mode_64 = arm64.get("KernelArmFastMode").unwrap();
    assert_eq!(&SingleValue::Boolean(false), fast_mode_64);

    let sabre = f.sel4.config.contextual_config.get("sabre").unwrap();
    assert_eq!(1, sabre.len());
    let arb_key_sabre = sabre.get("SomeOtherKey").unwrap();
    assert_eq!(&SingleValue::String("hi".to_owned()), arb_key_sabre);

    let some_arbitrary_platform = f
        .sel4
        .config
        .contextual_config
        .get("some_arbitrary_platform")
        .unwrap();
    assert_eq!(1, some_arbitrary_platform.len());
    let arb_key_some_arbitrary_platform = some_arbitrary_platform.get("SomeOtherKey").unwrap();
    assert_eq!(
        &SingleValue::String("aloha".to_owned()),
        arb_key_some_arbitrary_platform
    );

    let resolved_some_arbitrary_platform_default = contextualized::Contextualized::from_full(
        f.clone(),
        Arch::Arm,
        Sel4Arch::Aarch32,
        true,
        Platform("some_arbitrary_platform".to_owned()),
        None,
    )
    .unwrap();

    let resolved_sabre = contextualized::Contextualized::from_full(
        f.clone(),
        Arch::Arm,
        Sel4Arch::Aarch32,
        true,
        Platform("sabre".to_string()),
        None,
    )
    .unwrap();
    assert_ne!(resolved_some_arbitrary_platform_default, resolved_sabre);
}

#[test]
fn round_trip() {
    let f_alpha: full::Full = EXAMPLE.parse().expect("could not read toml");
    let serialized = f_alpha
        .to_toml_string()
        .expect("could not serialize to toml");
    let f_beta: full::Full = serialized.parse().expect("could not read serialized toml");
    assert_eq!(f_alpha, f_beta);
    assert_eq!(EXAMPLE, serialized);
}

#[test]
fn happy_path_straight_to_contextualized() {
    let f = contextualized::Contextualized::from_str(
        EXAMPLE,
        Arch::Arm,
        Sel4Arch::Aarch32,
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
    assert_eq!(Sel4Arch::Aarch32, f.context.sel4_arch);
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
