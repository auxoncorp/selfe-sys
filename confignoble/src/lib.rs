use serde::{Deserialize, Serialize};

use semver_parser::version::{parse as parse_version, Version as SemVersion};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use toml::de::Error as TomlDeError;
use toml::ser::{to_string_pretty, Error as TomlSerError};
use toml::value::{Table as TomlTable, Value as TomlValue};

const DEFAULT_CONFIG_CONTENT: &str = include_str!("../../default_config.toml");

/// Produce a unique instance of the default config content
pub fn get_default_config() -> full::Full {
    DEFAULT_CONFIG_CONTENT
        .parse()
        .map_err(|e| format!("{}", e))
        .expect("Default config content should always be valid.")
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Default)]
pub struct PlatformBuild {
    pub cross_compiler_prefix: Option<String>,
    pub toolchain_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SeL4Source {
    Version(SemVersion),
    LocalDirectories {
        kernel_dir: PathBuf,
        tools_dir: PathBuf,
    },
}

pub(crate) mod raw {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Raw {
        pub(crate) sel4: SeL4,
        pub(crate) build: Option<HashMap<String, PlatformBuild>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct SeL4 {
        pub(crate) kernel_dir: Option<PathBuf>,
        pub(crate) tools_dir: Option<PathBuf>,
        pub(crate) version: Option<String>,
        pub(crate) default_platform: Option<String>,
        pub(crate) config: HashMap<String, TomlValue>,
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum SingleValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl SingleValue {
    pub fn from_toml(t: TomlValue) -> Result<SingleValue, ImportError> {
        match t {
            TomlValue::String(s) => Ok(SingleValue::String(s)),
            TomlValue::Integer(i) => Ok(SingleValue::Integer(i)),
            TomlValue::Float(f) => Ok(SingleValue::Float(f)),
            TomlValue::Boolean(b) => Ok(SingleValue::Boolean(b)),
            TomlValue::Table(_) | TomlValue::Datetime(_) | TomlValue::Array(_) => {
                Err(ImportError::NonSingleValue {
                    found: t.type_str(),
                })
            }
        }
    }

    pub fn to_toml(&self) -> TomlValue {
        match self {
            SingleValue::String(s) => TomlValue::String(s.clone()),
            SingleValue::Integer(i) => TomlValue::Integer(*i),
            SingleValue::Float(f) => TomlValue::Float(*f),
            SingleValue::Boolean(b) => TomlValue::Boolean(*b),
        }
    }

    fn single_pair((k, v): (String, TomlValue)) -> Result<(String, SingleValue), ImportError> {
        let sv = SingleValue::from_toml(v).map_err(|e| match e {
            ImportError::NonSingleValue { found } => ImportError::TypeMismatch {
                name: k.clone(),
                expected: "a single string, integer, float, or boolean",
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
        pub build: HashMap<String, PlatformBuild>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SeL4 {
        pub source: SeL4Source,
        pub default_platform: Option<String>,
        pub config: Config,
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
        pub shared_config: HashMap<String, SingleValue>,
        pub debug_config: HashMap<String, SingleValue>,
        pub release_config: HashMap<String, SingleValue>,
        pub contextual_config: HashMap<String, HashMap<String, SingleValue>>,
    }

    impl std::str::FromStr for Full {
        type Err = ImportError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let raw::Raw { sel4, build } = toml::from_str(s)?;

            let source = match (sel4.kernel_dir, sel4.tools_dir, sel4.version) {
                (Some(kernel_dir), Some(tools_dir), None) => SeL4Source::LocalDirectories {
                    kernel_dir,
                    tools_dir,
                },
                (None, None, Some(version)) => SeL4Source::Version(
                    parse_version(&version).map_err(|_ve| ImportError::InvalidSeL4Source)?,
                ),
                (_, _, _) => return Err(ImportError::InvalidSeL4Source),
            };

            Ok(Full {
                sel4: SeL4 {
                    source,
                    default_platform: sel4.default_platform,
                    config: structure_config(sel4.config)?,
                },
                build: build.unwrap_or_else(|| HashMap::new()),
            })
        }
    }

    impl Full {
        pub fn to_toml_string(&self) -> Result<String, TomlSerError> {
            let mut sel4 = TomlTable::new();
            match &self.sel4.source {
                SeL4Source::Version(version) => {
                    sel4.insert(
                        "version".to_owned(),
                        TomlValue::String(format!("{}", version)),
                    );
                }
                SeL4Source::LocalDirectories {
                    kernel_dir,
                    tools_dir,
                } => {
                    sel4.insert(
                        "kernel_dir".to_owned(),
                        TomlValue::String(format!("{}", kernel_dir.display())),
                    );
                    sel4.insert(
                        "tools_dir".to_owned(),
                        TomlValue::String(format!("{}", tools_dir.display())),
                    );
                }
            }
            if let Some(plat) = &self.sel4.default_platform {
                sel4.insert(
                    "default_platform".to_owned(),
                    TomlValue::String(plat.to_owned()),
                );
            }
            let mut config = TomlTable::new();
            config.extend(
                self.sel4
                    .config
                    .shared_config
                    .iter()
                    .map(SingleValue::toml_pair),
            );
            if !self.sel4.config.debug_config.is_empty() {
                config.insert(
                    "debug".to_owned(),
                    TomlValue::Table(
                        self.sel4
                            .config
                            .debug_config
                            .iter()
                            .map(SingleValue::toml_pair)
                            .collect(),
                    ),
                );
            }
            if !self.sel4.config.release_config.is_empty() {
                config.insert(
                    "release".to_owned(),
                    TomlValue::Table(
                        self.sel4
                            .config
                            .release_config
                            .iter()
                            .map(SingleValue::toml_pair)
                            .collect(),
                    ),
                );
            }
            for (k, t) in self.sel4.config.contextual_config.iter() {
                config.insert(
                    k.to_owned(),
                    TomlValue::Table(t.iter().map(SingleValue::toml_pair).collect()),
                );
            }

            sel4.insert("config".to_owned(), TomlValue::Table(config));

            let mut top = TomlTable::new();
            top.insert("sel4".to_owned(), TomlValue::Table(sel4));
            to_string_pretty(&top)
        }
    }

    fn toml_table_to_map_of_singles(
        t: toml::value::Table,
    ) -> Result<HashMap<String, SingleValue>, ImportError> {
        t.into_iter().map(SingleValue::single_pair).collect()
    }

    fn structure_config(rc: HashMap<String, TomlValue>) -> Result<Config, ImportError> {
        let mut shared_config: HashMap<String, SingleValue> = HashMap::new();
        let mut debug_config: Option<HashMap<String, SingleValue>> = None;
        let mut release_config: Option<HashMap<String, SingleValue>> = None;
        let mut contextual_config: HashMap<String, HashMap<String, SingleValue>> = HashMap::new();
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
                    TomlValue::String(_)
                    | TomlValue::Integer(_)
                    | TomlValue::Float(_)
                    | TomlValue::Boolean(_) => {
                        let (k, v) = SingleValue::single_pair((k, v))?;
                        shared_config.insert(k, v);
                    }
                    TomlValue::Table(t) => {
                        contextual_config.insert(k, toml_table_to_map_of_singles(t)?);
                    }
                    TomlValue::Datetime(_) | TomlValue::Array(_) => {
                        return Err(ImportError::TypeMismatch {
                            name: k,
                            expected: "any toml type except array or datetime",
                            found: v.type_str(),
                        });
                    }
                }
            }
        }

        Ok(Config {
            shared_config,
            debug_config: debug_config.unwrap_or_else(HashMap::new),
            release_config: release_config.unwrap_or_else(HashMap::new),
            contextual_config,
        })
    }
}

pub mod contextualized {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Contextualized {
        pub sel4_source: SeL4Source,
        pub context: Context,
        pub sel4_config: HashMap<String, SingleValue>,
        pub build: PlatformBuild,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Context {
        pub target: String,
        pub platform: String,
        pub is_debug: bool,
    }

    impl Contextualized {
        pub fn from_str(
            source_toml: &str,
            target: String,
            is_debug: bool,
            platform: Option<String>,
        ) -> Result<Contextualized, ImportError> {
            let f: full::Full = source_toml.parse()?;
            Self::from_full(f, target, is_debug, platform)
        }

        pub fn from_full(
            mut source: full::Full,
            target: String,
            is_debug: bool,
            platform: Option<String>,
        ) -> Result<Contextualized, ImportError> {
            let platform = platform
                .or(source.sel4.default_platform)
                .ok_or_else(|| ImportError::NoPlatformSupplied)?;

            let build = source.build.remove(&platform).unwrap_or_default();

            let context = Context {
                platform,
                target,
                is_debug,
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
                sel4_source: source.sel4.source.clone(),
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
    NoPlatformSupplied,
    InvalidSeL4Source,
}

impl Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            ImportError::TomlDeserializeError(s) => f.write_fmt(format_args!("Error deserializing toml: {}", s)),
            ImportError::TypeMismatch { name, expected, found } => f.write_fmt(format_args!("Config toml contained a type mismatch for {}. Found {} when {} was expected", name, found, expected)),
            ImportError::NonSingleValue { found } => f.write_fmt(format_args!("Config toml contained a type problem where a singular value was expected but, {} was found", found)),
            ImportError::NoPlatformSupplied => f.write_fmt(format_args!("Config contextualization failed because no platform was supplied and no default was available.")),
            ImportError::InvalidSeL4Source => f.write_fmt(format_args!("Config toml's [sel4] table must contain either a single `version` property or both `kernel_dir` and `tools_dir` properties.")),
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
                minor: 0,
                patch: 0,
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
        match contextualized::Contextualized::from_full(f, "target".to_owned(), true, None) {
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
        let c =
            contextualized::Contextualized::from_full(f, "target".to_owned(), true, None).unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(true, c.context.is_debug);
        assert_eq!("target".to_owned(), c.context.target);
    }

    #[test]
    fn override_default_platform_contextualization() {
        let mut f = full::Full::empty();
        let expected = "sabre".to_owned();
        let default = "pc99".to_owned();
        f.sel4.default_platform = Some(default.clone());
        let c = contextualized::Contextualized::from_full(
            f,
            "target".to_owned(),
            false,
            Some(expected.clone()),
        )
        .unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(false, c.context.is_debug);
        assert_eq!("target".to_owned(), c.context.target);
    }
}
