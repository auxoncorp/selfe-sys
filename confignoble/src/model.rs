use semver_parser::version::{parse as parse_version, Version as SemVersion};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use toml::de::Error as TomlDeError;
use toml::ser::{to_string_pretty, Error as TomlSerError};
use toml::value::{Table as TomlTable, Value as TomlValue};

const DEFAULT_CONFIG_CONTENT: &str = include_str!("default_config.toml");

/// Produce a unique instance of the default config content
pub fn get_default_config() -> full::Full {
    DEFAULT_CONFIG_CONTENT
        .parse()
        .map_err(|e| format!("{}", e))
        .expect("Default config content should always be valid.")
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum SeL4Source {
    Version(SemVersion),
    LocalDirectories {
        kernel_dir: PathBuf,
        tools_dir: PathBuf,
        util_libs_dir: PathBuf,
    },
}

pub(crate) mod raw {
    use super::full::{PlatformBuild, PlatformBuildProfile};
    use super::*;

    pub(crate) struct Raw {
        pub(crate) sel4: SeL4,
        pub(crate) build: Option<BTreeMap<String, PlatformBuild>>,
    }

    pub(crate) struct SeL4 {
        pub(crate) kernel_dir: Option<PathBuf>,
        pub(crate) tools_dir: Option<PathBuf>,
        pub(crate) util_libs_dir: Option<PathBuf>,
        pub(crate) version: Option<String>,
        pub(crate) default_platform: Option<String>,
        pub(crate) config: BTreeMap<String, TomlValue>,
    }

    impl std::str::FromStr for Raw {
        type Err = ImportError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let top: TomlValue = toml::from_str(s)?;
            let top: &TomlTable = top.as_table().ok_or_else(|| ImportError::TypeMismatch {
                name: "top-level".to_string(),
                expected: "table",
                found: top.type_str(),
            })?;

            fn parse_sel4(table: &TomlTable) -> Result<SeL4, ImportError> {
                let kernel_dir = parse_optional_string(table, "kernel_dir")?.map(PathBuf::from);
                let tools_dir = parse_optional_string(table, "tools_dir")?.map(PathBuf::from);
                let util_libs_dir =
                    parse_optional_string(table, "util_libs_dir")?.map(PathBuf::from);
                let version = parse_optional_string(table, "version")?;
                let default_platform = parse_optional_string(table, "default_platform")?;
                let raw_config = table
                    .get("config")
                    .ok_or_else(|| ImportError::TypeMismatch {
                        name: "config".to_string(),
                        expected: "table",
                        found: "none",
                    })?;
                let config_table =
                    raw_config
                        .as_table()
                        .ok_or_else(|| ImportError::TypeMismatch {
                            name: "config".to_string(),
                            expected: "table",
                            found: raw_config.type_str(),
                        })?;
                let mut config = BTreeMap::new();
                for (k, v) in config_table.iter() {
                    config.insert(k.to_owned(), v.clone());
                }
                Ok(SeL4 {
                    kernel_dir,
                    tools_dir,
                    util_libs_dir,
                    version,
                    default_platform,
                    config,
                })
            }

            fn parse_optional_string(
                table: &TomlTable,
                key: &str,
            ) -> Result<Option<String>, ImportError> {
                if let Some(val) = table.get(key) {
                    Ok(Some(val.as_str().map(|s| s.to_owned()).ok_or_else(
                        || ImportError::TypeMismatch {
                            name: key.to_string(),
                            expected: "string",
                            found: val.type_str(),
                        },
                    )?))
                } else {
                    Ok(None)
                }
            }

            fn parse_required_string(table: &TomlTable, key: &str) -> Result<String, ImportError> {
                if let Some(val) = table.get(key) {
                    Ok(val.as_str().map(|s| s.to_owned()).ok_or_else(|| {
                        ImportError::TypeMismatch {
                            name: key.to_string(),
                            expected: "string",
                            found: val.type_str(),
                        }
                    })?)
                } else {
                    Err(ImportError::TypeMismatch {
                        name: key.to_string(),
                        expected: "string",
                        found: "none",
                    })
                }
            }

            fn parse_build(
                table: &TomlTable,
            ) -> Result<BTreeMap<String, PlatformBuild>, ImportError> {
                let mut map = BTreeMap::new();
                for (k, v) in table.iter() {
                    if let Some(plat_table) = v.as_table() {
                        map.insert(k.to_string(), parse_platform_build(plat_table)?);
                    } else {
                        return Err(ImportError::TypeMismatch {
                            name: k.to_string(),
                            expected: "table",
                            found: v.type_str(),
                        });
                    }
                }
                Ok(map)
            }
            fn parse_platform_build(table: &TomlTable) -> Result<PlatformBuild, ImportError> {
                let cross_compiler_prefix = parse_optional_string(table, "cross_compiler_prefix")?;
                let toolchain_dir =
                    parse_optional_string(table, "toolchain_dir")?.map(PathBuf::from);

                fn parse_build_profile(
                    parent_table: &TomlTable,
                    profile_name: &'static str,
                ) -> Result<Option<PlatformBuildProfile>, ImportError> {
                    if let Some(v) = parent_table.get(profile_name) {
                        if let Some(profile_table) = v.as_table() {
                            Ok(Some(PlatformBuildProfile {
                                make_root_task: parse_optional_string(
                                    profile_table,
                                    "make_root_task",
                                )?,
                                root_task_image: PathBuf::from(parse_required_string(
                                    profile_table,
                                    "root_task_image",
                                )?),
                            }))
                        } else {
                            return Err(ImportError::TypeMismatch {
                                name: profile_name.to_string(),
                                expected: "table",
                                found: v.type_str(),
                            });
                        }
                    } else {
                        Ok(None)
                    }
                }
                let debug_build_profile = parse_build_profile(table, "debug")?;
                let release_build_profile = parse_build_profile(table, "release")?;

                Ok(PlatformBuild {
                    cross_compiler_prefix,
                    toolchain_dir,
                    debug_build_profile,
                    release_build_profile,
                })
            }

            let sel4 = parse_sel4(top.get("sel4").and_then(TomlValue::as_table).ok_or_else(
                || ImportError::TypeMismatch {
                    name: "sel4".to_string(),
                    expected: "table",
                    found: "none",
                },
            )?)?;

            let build = if let Some(build_val) = top.get("build") {
                let build_table =
                    build_val
                        .as_table()
                        .ok_or_else(|| ImportError::TypeMismatch {
                            name: "build".to_string(),
                            expected: "table",
                            found: build_val.type_str(),
                        })?;
                Some(parse_build(build_table)?)
            } else {
                None
            };

            Ok(Raw { sel4, build })
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Hash)]
pub enum SingleValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl SingleValue {
    pub fn from_toml(t: TomlValue) -> Result<SingleValue, ImportError> {
        match t {
            TomlValue::String(s) => Ok(SingleValue::String(s)),
            TomlValue::Integer(i) => Ok(SingleValue::Integer(i)),
            TomlValue::Boolean(b) => Ok(SingleValue::Boolean(b)),
            TomlValue::Float(_)
            | TomlValue::Table(_)
            | TomlValue::Datetime(_)
            | TomlValue::Array(_) => Err(ImportError::NonSingleValue {
                found: t.type_str(),
            }),
        }
    }

    pub fn to_toml(&self) -> TomlValue {
        match self {
            SingleValue::String(s) => TomlValue::String(s.clone()),
            SingleValue::Integer(i) => TomlValue::Integer(*i),
            SingleValue::Boolean(b) => TomlValue::Boolean(*b),
        }
    }

    fn single_pair((k, v): (String, TomlValue)) -> Result<(String, SingleValue), ImportError> {
        let sv = SingleValue::from_toml(v).map_err(|e| match e {
            ImportError::NonSingleValue { found } => ImportError::TypeMismatch {
                name: k.clone(),
                expected: "a single string, integer, or boolean",
                found,
            },
            _ => e,
        })?;
        Ok((k, sv))
    }

    fn toml_pair((k, v): (&String, &SingleValue)) -> (String, TomlValue) {
        (k.to_owned(), v.to_toml())
    }
}

pub mod full {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Full {
        pub sel4: SeL4,
        pub build: BTreeMap<String, PlatformBuild>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SeL4 {
        pub source: SeL4Source,
        pub default_platform: Option<String>,
        pub config: Config,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Default, Hash)]
    pub struct PlatformBuild {
        pub cross_compiler_prefix: Option<String>,
        pub toolchain_dir: Option<PathBuf>,
        pub debug_build_profile: Option<PlatformBuildProfile>,
        pub release_build_profile: Option<PlatformBuildProfile>,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Default, Hash)]
    pub struct PlatformBuildProfile {
        pub make_root_task: Option<String>,
        pub root_task_image: PathBuf,
    }

    impl SeL4 {
        pub fn new(source: SeL4Source, default_platform: Option<String>, config: Config) -> Self {
            SeL4 {
                source,
                default_platform,
                config,
            }
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct Config {
        pub shared_config: BTreeMap<String, SingleValue>,
        pub debug_config: BTreeMap<String, SingleValue>,
        pub release_config: BTreeMap<String, SingleValue>,
        pub contextual_config: BTreeMap<String, BTreeMap<String, SingleValue>>,
    }

    impl std::str::FromStr for Full {
        type Err = ImportError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let raw::Raw { sel4, build } = s.parse()?;

            let source = match (
                sel4.kernel_dir,
                sel4.tools_dir,
                sel4.util_libs_dir,
                sel4.version,
            ) {
                (Some(kernel_dir), Some(tools_dir), Some(util_libs_dir), None) => {
                    SeL4Source::LocalDirectories {
                        kernel_dir,
                        tools_dir,
                        util_libs_dir,
                    }
                }
                (None, None, None, Some(version)) => SeL4Source::Version(
                    parse_version(&version).map_err(|_ve| ImportError::InvalidSeL4Source)?,
                ),
                (_, _, _, _) => return Err(ImportError::InvalidSeL4Source),
            };

            Ok(Full {
                sel4: SeL4 {
                    source,
                    default_platform: sel4.default_platform,
                    config: structure_config(sel4.config)?,
                },
                build: build.unwrap_or_else(|| BTreeMap::new()),
            })
        }
    }

    /// Helper extension trait to make toml generation a little less verbose
    trait TomlTableExt {
        fn insert_str<K: Into<String>, V: Into<String>>(
            &mut self,
            key: K,
            value: V,
        ) -> Option<TomlValue>;
        fn insert_table<K: Into<String>>(&mut self, key: K, value: TomlTable) -> Option<TomlValue>;
    }

    impl TomlTableExt for TomlTable {
        fn insert_str<K: Into<String>, V: Into<String>>(
            &mut self,
            key: K,
            value: V,
        ) -> Option<TomlValue> {
            self.insert(key.into(), TomlValue::String(value.into()))
        }

        fn insert_table<K: Into<String>>(&mut self, key: K, value: TomlTable) -> Option<TomlValue> {
            self.insert(key.into(), TomlValue::Table(value))
        }
    }

    impl Full {
        fn to_toml(&self) -> TomlTable {
            let mut sel4 = TomlTable::new();
            match &self.sel4.source {
                SeL4Source::Version(version) => {
                    sel4.insert_str("version", format!("{}", version));
                }
                SeL4Source::LocalDirectories {
                    kernel_dir,
                    tools_dir,
                    util_libs_dir,
                } => {
                    sel4.insert_str("kernel_dir", format!("{}", kernel_dir.display()));
                    sel4.insert_str("tools_dir", format!("{}", tools_dir.display()));
                    sel4.insert_str("util_libs_dir", format!("{}", util_libs_dir.display()));
                }
            }

            if let Some(plat) = &self.sel4.default_platform {
                sel4.insert_str("default_platform", plat.as_ref());
            }

            fn serialize_config(sel4_config: &Config) -> TomlTable {
                let mut config = TomlTable::new();
                config.extend(sel4_config.shared_config.iter().map(SingleValue::toml_pair));
                if !sel4_config.debug_config.is_empty() {
                    config.insert_table(
                        "debug",
                        sel4_config
                            .debug_config
                            .iter()
                            .map(SingleValue::toml_pair)
                            .collect(),
                    );
                }
                if !sel4_config.release_config.is_empty() {
                    config.insert_table(
                        "release",
                        sel4_config
                            .release_config
                            .iter()
                            .map(SingleValue::toml_pair)
                            .collect(),
                    );
                }
                for (k, t) in sel4_config.contextual_config.iter() {
                    config.insert_table(k.as_ref(), t.iter().map(SingleValue::toml_pair).collect());
                }
                config
            }
            sel4.insert_table("config", serialize_config(&self.sel4.config));

            fn serialize_build(source: &BTreeMap<String, PlatformBuild>) -> Option<TomlTable> {
                if source.is_empty() {
                    return None;
                }
                let mut build = TomlTable::new();
                for (k, plat) in source.iter() {
                    let mut plat_table = TomlTable::new();
                    if let Some(ref v) = plat.cross_compiler_prefix {
                        plat_table.insert_str("cross_compiler_prefix", v.as_ref());
                    }
                    if let Some(ref v) = plat.toolchain_dir {
                        plat_table.insert_str("toolchain_dir", format!("{}", v.display()));
                    }

                    fn serialize_profile_build(
                        source: &Option<PlatformBuildProfile>,
                    ) -> Option<TomlTable> {
                        source.as_ref().map(|v| {
                            let mut prof_table = TomlTable::new();
                            if let Some(mrt) = v.make_root_task.as_ref() {
                                prof_table.insert_str("make_root_task", mrt.as_ref());
                            }
                            prof_table.insert_str(
                                "root_task_image",
                                format!("{}", v.root_task_image.display()),
                            );
                            prof_table
                        })
                    }
                    if let Some(t) = serialize_profile_build(&plat.debug_build_profile) {
                        plat_table.insert_table("debug", t);
                    }
                    if let Some(t) = serialize_profile_build(&plat.release_build_profile) {
                        plat_table.insert_table("release", t);
                    }
                    build.insert_table(k.as_ref(), plat_table);
                }
                Some(build)
            }

            let mut top = TomlTable::new();
            top.insert_table("sel4", sel4);
            if let Some(build) = serialize_build(&self.build) {
                top.insert_table("build", build);
            }
            top
        }

        /// Serialize the full contents to a toml string
        pub fn to_toml_string(&self) -> Result<String, TomlSerError> {
            to_string_pretty(&self.to_toml())
        }
    }

    fn toml_table_to_map_of_singles(
        t: toml::value::Table,
    ) -> Result<BTreeMap<String, SingleValue>, ImportError> {
        t.into_iter().map(SingleValue::single_pair).collect()
    }

    fn structure_config(rc: BTreeMap<String, TomlValue>) -> Result<Config, ImportError> {
        let mut shared_config: BTreeMap<String, SingleValue> = BTreeMap::new();
        let mut debug_config: Option<BTreeMap<String, SingleValue>> = None;
        let mut release_config: Option<BTreeMap<String, SingleValue>> = None;
        let mut contextual_config: BTreeMap<String, BTreeMap<String, SingleValue>> =
            BTreeMap::new();
        for (k, v) in rc.into_iter() {
            if k == "debug" {
                match v {
                    TomlValue::Table(t) => {
                        debug_config.replace(toml_table_to_map_of_singles(t)?);
                    }
                    _ => {
                        return Err(ImportError::TypeMismatch {
                            name: k,
                            expected: "table",
                            found: v.type_str(),
                        });
                    }
                }
                continue;
            } else if k == "release" {
                match v {
                    TomlValue::Table(t) => {
                        release_config.replace(toml_table_to_map_of_singles(t)?);
                    }
                    _ => {
                        return Err(ImportError::TypeMismatch {
                            name: k,
                            expected: "table",
                            found: v.type_str(),
                        });
                    }
                }
                continue;
            } else {
                match v {
                    TomlValue::String(_) | TomlValue::Integer(_) | TomlValue::Boolean(_) => {
                        let (k, v) = SingleValue::single_pair((k, v))?;
                        shared_config.insert(k, v);
                    }
                    TomlValue::Table(t) => {
                        contextual_config.insert(k, toml_table_to_map_of_singles(t)?);
                    }
                    TomlValue::Float(_) | TomlValue::Datetime(_) | TomlValue::Array(_) => {
                        return Err(ImportError::TypeMismatch {
                            name: k,
                            expected: "any toml type except float, array, or datetime",
                            found: v.type_str(),
                        });
                    }
                }
            }
        }

        Ok(Config {
            shared_config,
            debug_config: debug_config.unwrap_or_else(BTreeMap::new),
            release_config: release_config.unwrap_or_else(BTreeMap::new),
            contextual_config,
        })
    }
}

trait RelativePath {
    // If self is relative, and a base is supplied, evaluate self relative to
    // the base. Otherwise hand back self.
    fn relative_to(&self, base: Option<&Path>) -> PathBuf;
}

impl RelativePath for Path {
    fn relative_to(&self, base: Option<&Path>) -> PathBuf {
        if self.is_relative() {
            match base {
                Some(p) => p.join(self),
                None => self.to_path_buf(),
            }
        } else {
            self.to_path_buf()
        }
    }
}

pub mod contextualized {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Hash)]
    pub struct Contextualized {
        pub sel4_source: SeL4Source,
        pub context: Context,
        pub sel4_config: BTreeMap<String, SingleValue>,
        pub build: Build,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Default, Hash)]
    pub struct Build {
        pub cross_compiler_prefix: Option<String>,
        pub toolchain_dir: Option<PathBuf>,
        pub root_task: Option<RootTask>,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Default, Hash)]
    pub struct RootTask {
        pub make_command: Option<String>,
        pub image_path: PathBuf,
    }

    #[derive(Debug, Clone, PartialEq, Hash)]
    pub struct Context {
        pub target: String,
        pub platform: String,
        pub is_debug: bool,
        pub base_dir: Option<PathBuf>,
    }

    impl Contextualized {
        pub fn from_str(
            source_toml: &str,
            target: String,
            is_debug: bool,
            platform: Option<String>,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let f: full::Full = source_toml.parse()?;
            Self::from_full(f, target, is_debug, platform, base_dir)
        }

        pub fn from_full(
            mut source: full::Full,
            target: String,
            is_debug: bool,
            platform: Option<String>,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let platform = platform
                .or(source.sel4.default_platform)
                .ok_or_else(|| ImportError::NoPlatformSupplied)?;
            let context = Context {
                platform: platform.clone(),
                target,
                is_debug,
                base_dir: base_dir.map(|p| p.to_path_buf()),
            };

            let platform_build =
                source
                    .build
                    .remove(&platform)
                    .ok_or_else(|| ImportError::NoBuildSupplied {
                        platform: platform.clone(),
                        profile: if is_debug { "debug" } else { "release " },
                    })?;
            let build_profile = if is_debug {
                platform_build.debug_build_profile
            } else {
                platform_build.release_build_profile
            };
            let root_task = build_profile.map(|bp| RootTask {
                make_command: bp.make_root_task,
                image_path: bp.root_task_image.relative_to(base_dir),
            });
            let build = Build {
                cross_compiler_prefix: platform_build.cross_compiler_prefix,
                toolchain_dir: platform_build
                    .toolchain_dir
                    .map(|p| p.relative_to(base_dir)),
                root_task,
            };

            let source_config = source.sel4.config;
            let mut sel4_config = source_config.shared_config;
            if is_debug {
                sel4_config.extend(source_config.debug_config)
            } else {
                sel4_config.extend(source_config.release_config)
            }
            let mut source_contextual_config = source_config.contextual_config;
            if let Some(target_config) = source_contextual_config.remove(&context.target) {
                sel4_config.extend(target_config);
            }
            if let Some(platform_config) = source_contextual_config.remove(&context.platform) {
                sel4_config.extend(platform_config);
            }

            Ok(Contextualized {
                sel4_source: match source.sel4.source {
                    SeL4Source::Version(v) => SeL4Source::Version(v),
                    SeL4Source::LocalDirectories {
                        kernel_dir,
                        tools_dir,
                        util_libs_dir,
                    } => SeL4Source::LocalDirectories {
                        kernel_dir: kernel_dir.relative_to(base_dir),
                        tools_dir: tools_dir.relative_to(base_dir),
                        util_libs_dir: util_libs_dir.relative_to(base_dir),
                    },
                },
                context,
                sel4_config,
                build,
            })
        }

        pub fn print_boolean_feature_flags(&self) {
            for (k, v) in self.sel4_config.iter() {
                match v {
                    SingleValue::Boolean(true) => println!("cargo:rustc-cfg={}", k),
                    _ => (),
                };
            }
        }
    }
}

#[derive(Debug)]
pub enum ImportError {
    TomlDeserializeError(String),
    TypeMismatch {
        name: String,
        expected: &'static str,
        found: &'static str,
    },
    NonSingleValue {
        found: &'static str,
    },
    InvalidSeL4Source,
    NoPlatformSupplied,
    NoBuildSupplied {
        platform: String,
        profile: &'static str,
    },
}

impl Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            ImportError::TomlDeserializeError(s) => f.write_fmt(format_args!("Error deserializing toml: {}", s)),
            ImportError::TypeMismatch { name, expected, found } => f.write_fmt(format_args!("Config toml contained a type mismatch for {}. Found {} when {} was expected", name, found, expected)),
            ImportError::NonSingleValue { found } => f.write_fmt(format_args!("Config toml contained a type problem where a singular value was expected but, {} was found", found)),
            ImportError::NoPlatformSupplied => f.write_fmt(format_args!("Config contextualization failed because no platform was supplied and no default was available.")),
            ImportError::InvalidSeL4Source => f.write_fmt(format_args!("Config toml's [sel4] table must contain either a single `version` property or all of the `kernel_dir`, `tools_dir`, and `util_libs_dir` properties.")),
            ImportError::NoBuildSupplied { platform, profile } => f.write_fmt(format_args!("Config toml must contain a [build.platform.profile] table like [build.{}.{}] but none was supplied.", platform, profile)),
        }
    }
}

impl From<TomlDeError> for ImportError {
    fn from(tde: TomlDeError) -> Self {
        ImportError::TomlDeserializeError(tde.description().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    impl full::Full {
        fn empty() -> Self {
            full::Full {
                sel4: full::SeL4 {
                    source: SeL4Source::LocalDirectories {
                        kernel_dir: PathBuf::from("."),
                        tools_dir: PathBuf::from("."),
                        util_libs_dir: PathBuf::from("."),
                    },
                    default_platform: None,
                    config: Default::default(),
                },
                build: Default::default(),
            }
        }
    }

    #[test]
    fn default_content_is_valid() {
        let f: full::Full = get_default_config();

        assert_eq!(
            SeL4Source::Version(SemVersion {
                major: 10,
                minor: 1,
                patch: 1,
                pre: vec![],
                build: vec![],
            }),
            f.sel4.source
        )
    }

    #[test]
    fn platform_required_for_contextualization() {
        let f = full::Full::empty();
        assert_eq!(&None, &f.sel4.default_platform);
        match contextualized::Contextualized::from_full(f, "target".to_owned(), true, None, None) {
            Ok(_) => panic!("Expected an Err about missing platform"),
            Err(e) => match e {
                ImportError::NoPlatformSupplied => (), // All according to plan
                _ => panic!("Unexpected Err kind"),
            },
        }
    }

    #[test]
    fn can_use_default_platform_contextualization() {
        let mut f = full::Full::empty();
        let expected = "pc99".to_owned();
        f.sel4.default_platform = Some(expected.clone());
        f.build.insert(
            expected.clone(),
            full::PlatformBuild {
                cross_compiler_prefix: None,
                toolchain_dir: None,
                debug_build_profile: Some(full::PlatformBuildProfile {
                    make_root_task: Some("cmake".to_string()),
                    root_task_image: PathBuf::from("over_here"),
                }),
                release_build_profile: None,
            },
        );
        let c = contextualized::Contextualized::from_full(f, "target".to_owned(), true, None, None)
            .unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(true, c.context.is_debug);
        assert_eq!("target".to_owned(), c.context.target);
        assert_eq!(
            "cmake",
            c.build
                .root_task
                .as_ref()
                .unwrap()
                .make_command
                .as_ref()
                .unwrap()
        );
        assert_eq!(
            PathBuf::from("over_here"),
            c.build.root_task.as_ref().unwrap().image_path
        );
    }

    #[test]
    fn override_default_platform_contextualization() {
        let mut f = full::Full::empty();
        let expected = "sabre".to_owned();
        let default = "pc99".to_owned();
        f.sel4.default_platform = Some(default.clone());
        f.build.insert(
            expected.clone(),
            full::PlatformBuild {
                cross_compiler_prefix: None,
                toolchain_dir: None,
                debug_build_profile: None,
                release_build_profile: Some(full::PlatformBuildProfile {
                    make_root_task: Some("cmake".to_string()),
                    root_task_image: PathBuf::from("over_here"),
                }),
            },
        );
        let c = contextualized::Contextualized::from_full(
            f,
            "target".to_owned(),
            false,
            Some(expected.clone()),
            None,
        )
        .unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(false, c.context.is_debug);
        assert_eq!("target".to_owned(), c.context.target);
        assert_eq!(
            "cmake",
            c.build
                .root_task
                .as_ref()
                .unwrap()
                .make_command
                .as_ref()
                .unwrap()
        );
        assert_eq!(
            PathBuf::from("over_here"),
            c.build.root_task.unwrap().image_path
        );
    }
}
