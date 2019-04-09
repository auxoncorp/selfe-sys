use confignoble::*;
use std::path::PathBuf;

const MINIMAL_EXAMPLE: &str = r#"[sel4]
default_platform = 'some_arbitrary_platform'
kernel_dir = './deps/seL4'
tools_dir = './deps/seL4_tools'

[sel4.config]
KernelRetypeFanOutLimit = 256

[sel4.config.arm32]
KernelArmFastMode = true

[sel4.config.arm64]
KernelArmFastMode = false

[sel4.config.debug]
KernelPrinting = true

[sel4.config.release]
KernelPrinting = false

[sel4.config.sabre]
SomeOtherKey = 'hi'

[sel4.config.some_arbitrary_platform]
SomeOtherKey = 'aloha'
"#;

#[test]
fn reads_from_external_default_file_okay() {
    let toml_content = include_str!("../../default_config.toml");
    let f: full::Full = toml_content.parse().expect("could not read toml");
    assert!(f.sel4.config.shared_config.len() > 0);
}

#[test]
fn full_parse_happy_path() {
    let f: full::Full = MINIMAL_EXAMPLE
        .parse()
        .expect("could not read toml to full");
    assert_eq!(PathBuf::from("./deps/seL4"), f.sel4.kernel_dir);
    assert_eq!(PathBuf::from("./deps/seL4_tools"), f.sel4.tools_dir);
    assert_eq!(
        Some("some_arbitrary_platform".to_owned()),
        f.sel4.default_platform
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

    let arm32 = f.sel4.config.contextual_config.get("arm32").unwrap();
    assert_eq!(1, arm32.len());
    let fast_mode_32 = arm32.get("KernelArmFastMode").unwrap();
    assert_eq!(&SingleValue::Boolean(true), fast_mode_32);

    let arm64 = f.sel4.config.contextual_config.get("arm64").unwrap();
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

    let resolved_some_arbitrary_platform_default =
        contextualized::Contextualized::from_full(f.clone(), "arm32".to_owned(), true, None)
            .unwrap();
    let resolved_sabre = contextualized::Contextualized::from_full(
        f.clone(),
        "arm32".to_owned(),
        true,
        Some("sabre".to_owned()),
    )
    .unwrap();
    assert_ne!(resolved_some_arbitrary_platform_default, resolved_sabre);
}

#[test]
fn round_trip() {
    let f: full::Full = MINIMAL_EXAMPLE.parse().expect("could not read toml");
    let serialized = f.to_toml_string().expect("could not serialize to toml");
    assert_eq!(MINIMAL_EXAMPLE, serialized);
}

#[test]
fn happy_path_straight_to_contextualized() {
    let f = contextualized::Contextualized::from_str(
        MINIMAL_EXAMPLE,
        "arm32".to_owned(),
        true,
        Some("sabre".to_owned()),
    )
    .unwrap();
    assert_eq!(PathBuf::from("./deps/seL4"), f.kernel_dir);
    assert_eq!(PathBuf::from("./deps/seL4_tools"), f.tools_dir);
    assert_eq!("arm32".to_owned(), f.context.target);
    assert_eq!("sabre".to_owned(), f.context.platform);
    assert_eq!(true, f.context.is_debug);
    assert_eq!(4, f.sel4_config.len());
    assert_eq!(
        &SingleValue::Integer(256),
        f.sel4_config.get("KernelRetypeFanOutLimit").unwrap()
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
