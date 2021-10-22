use super::full;
use super::{GitTarget, RepoSource, SeL4Sources, SingleValue};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use toml::de::Error as TomlDeError;
use toml::value::{Table as TomlTable, Value as TomlValue};

/// Internal intermediate representation to ease parsing of the toml format
pub(crate) struct Raw {
    pub(crate) sel4: RawSeL4,
    pub(crate) build: Option<BTreeMap<String, full::PlatformBuild>>,
    pub(crate) metadata: BTreeMap<String, TomlValue>,
}

/// Internal intermediate representation of the sel4 portion of the toml format
pub(crate) struct RawSeL4 {
    pub(crate) kernel: TomlTable,
    pub(crate) tools: TomlTable,
    pub(crate) util_libs: TomlTable,
    pub(crate) config: BTreeMap<String, TomlValue>,
}

/// The things that can go wrong when attempting to import this configuration format
#[derive(Debug)]
pub enum ImportError {
    TomlDeserializeError(String),
    TypeMismatch {
        name: String,
        expected: &'static str,
        found: &'static str,
    },
    MissingProperty {
        name: String,
        expected_type: &'static str,
    },
    NonSingleValue {
        found: &'static str,
    },
    UnsupportedProperties {
        extra_keys: Vec<String>,
    },
    InvalidSeL4Source,
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
            ImportError::MissingProperty{  name, expected_type } => f.write_fmt(format_args!("Config toml missing {}, expected to be of type {}", name, expected_type)),
            ImportError::NonSingleValue { found } => f.write_fmt(format_args!("Config toml contained a type problem where a singular value was expected but, {} was found", found)),
            ImportError::UnsupportedProperties { extra_keys } => f.write_fmt(format_args!("Config toml contained superfluous unsupported properties: {:?}.", extra_keys )),
            ImportError::InvalidSeL4Source => f.write_fmt(format_args!("Config toml's [sel4] table must contain either a single `version` property or all of the `kernel_dir`, `tools_dir`, and `util_libs_dir` properties.")),
            ImportError::NoBuildSupplied { platform, profile } => f.write_fmt(format_args!("Config toml must contain a [build.platform.profile] table like [build.{}.{}] but none was supplied.", platform, profile)),
        }
    }
}

impl From<TomlDeError> for ImportError {
    fn from(tde: TomlDeError) -> Self {
        ImportError::TomlDeserializeError(tde.to_string())
    }
}

impl FromStr for Raw {
    type Err = ImportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let top: TomlValue = toml::from_str(s)?;
        let top: &TomlTable = top.as_table().ok_or_else(|| ImportError::TypeMismatch {
            name: "top-level".to_string(),
            expected: "table",
            found: top.type_str(),
        })?;

        fn parse_sel4(table: &TomlTable) -> Result<RawSeL4, ImportError> {
            let kernel = parse_required_table(table, "kernel")?;
            let tools = parse_required_table(table, "tools")?;
            let util_libs = parse_required_table(table, "util_libs")?;

            let mut config = BTreeMap::new();
            if let Some(config_val) = table.get("config") {
                let raw_config =
                    config_val
                        .as_table()
                        .ok_or_else(|| ImportError::TypeMismatch {
                            name: "config".to_string(),
                            expected: "table",
                            found: config_val.type_str(),
                        })?;
                for (k, v) in raw_config.iter() {
                    config.insert(k.to_owned(), v.clone());
                }
            }
            Ok(RawSeL4 {
                kernel,
                tools,
                util_libs,
                config,
            })
        }

        fn parse_required_table(parent: &TomlTable, key: &str) -> Result<TomlTable, ImportError> {
            if let Some(val) = parent.get(key) {
                Ok(val.as_table().map(ToOwned::to_owned).ok_or_else(|| {
                    ImportError::TypeMismatch {
                        name: key.to_string(),
                        expected: "table",
                        found: val.type_str(),
                    }
                })?)
            } else {
                Err(ImportError::MissingProperty {
                    name: key.to_string(),
                    expected_type: "table",
                })
            }
        }

        fn parse_build(
            table: &TomlTable,
        ) -> Result<BTreeMap<String, full::PlatformBuild>, ImportError> {
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
        fn parse_platform_build(table: &TomlTable) -> Result<full::PlatformBuild, ImportError> {
            let cross_compiler_prefix = parse_optional_string(table, "cross_compiler_prefix")?;
            let toolchain_dir = parse_optional_string(table, "toolchain_dir")?.map(PathBuf::from);

            fn parse_build_profile(
                parent_table: &TomlTable,
                profile_name: &'static str,
            ) -> Result<Option<full::PlatformBuildProfile>, ImportError> {
                if let Some(v) = parent_table.get(profile_name) {
                    if let Some(profile_table) = v.as_table() {
                        Ok(Some(full::PlatformBuildProfile {
                            make_root_task: parse_optional_string(profile_table, "make_root_task")?,
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

            Ok(full::PlatformBuild {
                cross_compiler_prefix,
                toolchain_dir,
                debug_build_profile,
                release_build_profile,
            })
        }

        let sel4 = parse_sel4(
            top.get("sel4")
                .and_then(TomlValue::as_table)
                .ok_or_else(|| ImportError::MissingProperty {
                    name: "sel4".to_string(),
                    expected_type: "table",
                })?,
        )?;

        let build = if let Some(build_val) = top.get("build") {
            let build_table = build_val
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

        let mut metadata = BTreeMap::new();
        if let Some(metadata_val) = top.get("metadata") {
            let raw_metadata =
                metadata_val
                    .as_table()
                    .ok_or_else(|| ImportError::TypeMismatch {
                        name: "metadata".to_string(),
                        expected: "table",
                        found: metadata_val.type_str(),
                    })?;
            for (k, v) in raw_metadata.iter() {
                metadata.insert(k.to_owned(), v.clone());
            }
        }

        Ok(Raw {
            sel4,
            build,
            metadata,
        })
    }
}

impl SingleValue {
    pub fn from_toml(t: &TomlValue) -> Result<SingleValue, ImportError> {
        match t {
            TomlValue::String(s) => Ok(SingleValue::String(s.clone())),
            TomlValue::Integer(i) => Ok(SingleValue::Integer(*i)),
            TomlValue::Boolean(b) => Ok(SingleValue::Boolean(*b)),
            TomlValue::Float(_)
            | TomlValue::Table(_)
            | TomlValue::Datetime(_)
            | TomlValue::Array(_) => Err(ImportError::NonSingleValue {
                found: t.type_str(),
            }),
        }
    }
    fn single_pair((k, v): (&String, &TomlValue)) -> Result<(String, SingleValue), ImportError> {
        let sv = SingleValue::from_toml(v).map_err(|e| match e {
            ImportError::NonSingleValue { found } => ImportError::TypeMismatch {
                name: k.clone(),
                expected: "a single string, integer, or boolean",
                found,
            },
            _ => e,
        })?;
        Ok((k.clone(), sv))
    }
}

impl FromStr for full::Full {
    type Err = ImportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Raw {
            sel4,
            build,
            metadata,
        } = s.parse()?;
        let sources = SeL4Sources {
            kernel: parse_repo_source(&sel4.kernel)?,
            tools: parse_repo_source(&sel4.tools)?,
            util_libs: parse_repo_source(&sel4.util_libs)?,
        };

        Ok(full::Full {
            sel4: full::SeL4 {
                sources,
                config: structure_property_tree(sel4.config)?,
            },
            build: build.unwrap_or_else(BTreeMap::new),
            metadata: structure_property_tree(metadata)?,
        })
    }
}

fn parse_optional_string(table: &TomlTable, key: &str) -> Result<Option<String>, ImportError> {
    if let Some(val) = table.get(key) {
        Ok(Some(val.as_str().map(ToOwned::to_owned).ok_or_else(
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
        Ok(val
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| ImportError::TypeMismatch {
                name: key.to_string(),
                expected: "string",
                found: val.type_str(),
            })?)
    } else {
        Err(ImportError::MissingProperty {
            name: key.to_string(),
            expected_type: "string",
        })
    }
}

fn parse_repo_source(table: &TomlTable) -> Result<RepoSource, ImportError> {
    let path = parse_optional_string(table, "path")?;
    if let Some(path) = path {
        if table.len() > 1 {
            let extra_keys = table
                .iter()
                .filter_map(|(k, _v)| {
                    if k != "path" {
                        Some(k.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            return Err(ImportError::UnsupportedProperties { extra_keys });
        }
        Ok(RepoSource::LocalPath(PathBuf::from(path)))
    } else {
        let url = parse_required_string(table, "git")?;
        let branch = parse_optional_string(table, "branch")?;
        let tag = parse_optional_string(table, "tag")?;
        let rev = parse_optional_string(table, "rev")?;
        match (branch, tag, rev) {
            (Some(b), None, None) => Ok(RepoSource::RemoteGit {
                url,
                target: GitTarget::Branch(b.to_owned()),
            }),
            (None, Some(t), None) => Ok(RepoSource::RemoteGit {
                url,
                target: GitTarget::Tag(t.to_owned()),
            }),
            (None, None, Some(r)) => Ok(RepoSource::RemoteGit {
                url,
                target: GitTarget::Rev(r.to_owned()),
            }),
            _ => Err(ImportError::MissingProperty {
                name: "branch or tag or rev".to_string(),
                expected_type: "string",
            }),
        }
    }
}

fn structure_property_tree(
    rc: BTreeMap<String, TomlValue>,
) -> Result<full::PropertiesTree, ImportError> {
    let mut shared: BTreeMap<String, SingleValue> = BTreeMap::new();
    let mut debug: Option<BTreeMap<String, SingleValue>> = None;
    let mut release: Option<BTreeMap<String, SingleValue>> = None;
    let mut contextual: BTreeMap<String, BTreeMap<String, SingleValue>> = BTreeMap::new();
    for (k, v) in rc.into_iter() {
        if k == "debug" {
            match v {
                TomlValue::Table(t) => {
                    debug.replace(toml_table_to_map_of_singles(&t)?);
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
                    release.replace(toml_table_to_map_of_singles(&t)?);
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
                    let (k, v) = SingleValue::single_pair((&k, &v))?;
                    shared.insert(k, v);
                }
                TomlValue::Table(t) => {
                    contextual.insert(k, toml_table_to_map_of_singles(&t)?);
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

    Ok(full::PropertiesTree {
        shared,
        debug: debug.unwrap_or_else(BTreeMap::new),
        release: release.unwrap_or_else(BTreeMap::new),
        contextual,
    })
}

fn toml_table_to_map_of_singles(
    t: &toml::value::Table,
) -> Result<BTreeMap<String, SingleValue>, ImportError> {
    t.into_iter().map(SingleValue::single_pair).collect()
}
