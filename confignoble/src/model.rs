use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;
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

/// An enum-ified version of the rust's notion of arch, the first part of a rust target triple
#[derive(Copy, Clone)]
pub enum RustArch {
    Aarch64,
    Arm,
    Armebv7r,
    Armv5te,
    Armv7,
    Armv7r,
    Armv7s,
    Asmjs,
    I386,
    I586,
    I686,
    Mips,
    Mips64,
    Mips64el,
    Mipsel,
    Nvptx64,
    Powerpc,
    Powerpc64,
    Powerpc64le,
    Riscv32imac,
    Riscv32imc,
    Riscv64gc,
    Riscv64imac,
    S390x,
    Sparc64,
    Sparcv9,
    Thumbv6m,
    Thumbv7em,
    Thumbv7m,
    Thumbv7neon,
    Thumbv8mmain,
    Wasm32,
    X86_64,
}

impl FromStr for RustArch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch64" => Ok(RustArch::Aarch64),
            "arm" => Ok(RustArch::Arm),
            "armebv7r" => Ok(RustArch::Armebv7r),
            "armv5te" => Ok(RustArch::Armv5te),
            "armv7" => Ok(RustArch::Armv7),
            "armv7r" => Ok(RustArch::Armv7r),
            "armv7s" => Ok(RustArch::Armv7s),
            "asmjs" => Ok(RustArch::Asmjs),
            "i386" => Ok(RustArch::I386),
            "i586" => Ok(RustArch::I586),
            "i686" => Ok(RustArch::I686),
            "mips" => Ok(RustArch::Mips),
            "mips64" => Ok(RustArch::Mips64),
            "mips64el" => Ok(RustArch::Mips64el),
            "mipsel" => Ok(RustArch::Mipsel),
            "nvptx64" => Ok(RustArch::Nvptx64),
            "powerpc" => Ok(RustArch::Powerpc),
            "powerpc64" => Ok(RustArch::Powerpc64),
            "powerpc64le" => Ok(RustArch::Powerpc64le),
            "riscv32imac" => Ok(RustArch::Riscv32imac),
            "riscv32imc" => Ok(RustArch::Riscv32imc),
            "riscv64gc" => Ok(RustArch::Riscv64gc),
            "riscv64imac" => Ok(RustArch::Riscv32imc),
            "s390x" => Ok(RustArch::S390x),
            "sparc64" => Ok(RustArch::Sparc64),
            "sparcv9" => Ok(RustArch::Sparcv9),
            "thumbv6m" => Ok(RustArch::Thumbv6m),
            "thumbv7em" => Ok(RustArch::Thumbv7em),
            "thumbv7m" => Ok(RustArch::Thumbv7m),
            "thumbv7neon" => Ok(RustArch::Thumbv7neon),
            "thumbv8m.main" => Ok(RustArch::Thumbv8mmain),
            "wasm32" => Ok(RustArch::Wasm32),
            "x86_64" => Ok(RustArch::X86_64),
            _ => Err("Unrecognized rust arch".to_string()),
        }
    }
}

///  This is sel4's notion of 'sel4_arch'
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Sel4Arch {
    Aarch32,
    Aarch64,
    ArmHyp,
    Ia32,
    X86_64,
    Riscv32,
    Riscv64,
}

impl FromStr for Sel4Arch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch32" => Ok(Sel4Arch::Aarch32),
            "aarch64" => Ok(Sel4Arch::Aarch64),
            "arm_hyp" => Ok(Sel4Arch::ArmHyp),
            "ia32" => Ok(Sel4Arch::Ia32),
            "riscv32" => Ok(Sel4Arch::Riscv32),
            "riscv64" => Ok(Sel4Arch::Riscv64),
            "x86_64" => Ok(Sel4Arch::X86_64),
            _ => Err("Unrecognized sel4_arch".to_string()),
        }
    }
}

impl Sel4Arch {
    /// Create an Arch from the first part of a rust target triple
    pub fn from_rust_arch(rust_arch: RustArch) -> Option<Sel4Arch> {
        match rust_arch {
            RustArch::Aarch64 => Some(Sel4Arch::Aarch64),

            RustArch::Arm
            | RustArch::Armebv7r
            | RustArch::Armv7
            | RustArch::Armv7r
            | RustArch::Armv7s => Some(Sel4Arch::Aarch32),

            RustArch::I386 | RustArch::I586 | RustArch::I686 => Some(Sel4Arch::Ia32),

            RustArch::Riscv32imac | RustArch::Riscv32imc => Some(Sel4Arch::Riscv32),

            RustArch::Riscv64gc | RustArch::Riscv64imac => Some(Sel4Arch::Riscv64),

            RustArch::Thumbv6m
            | RustArch::Thumbv7em
            | RustArch::Thumbv7m
            | RustArch::Thumbv7neon => Some(Sel4Arch::Aarch32),

            RustArch::Thumbv8mmain => Some(Sel4Arch::Aarch64),

            RustArch::X86_64 => Some(Sel4Arch::X86_64),
            _ => None,
        }
    }
}

impl Display for Sel4Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Sel4Arch::Aarch32 => "aarch32",
            Sel4Arch::Aarch64 => "aarch64",
            Sel4Arch::ArmHyp => "arm_hyp",
            Sel4Arch::Ia32 => "ia32",
            Sel4Arch::X86_64 => "x86_64",
            Sel4Arch::Riscv32 => "riscv32",
            Sel4Arch::Riscv64 => "riscv64",
        };
        write!(f, "{}", s)
    }
}

/// This is sel4'x notion of 'arch'
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Arch {
    Arm,
    X86,
    Riscv,
}

impl FromStr for Arch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "arm" => Ok(Arch::Arm),
            "x86" => Ok(Arch::X86),
            "riscv" => Ok(Arch::Riscv),
            _ => Err("Unrecognized arch".to_string()),
        }
    }
}

impl Arch {
    pub fn from_sel4_arch(sel4_arch: Sel4Arch) -> Arch {
        match sel4_arch {
            Sel4Arch::Aarch32 | Sel4Arch::Aarch64 | Sel4Arch::ArmHyp => Arch::Arm,
            Sel4Arch::Ia32 | Sel4Arch::X86_64 => Arch::X86,
            Sel4Arch::Riscv32 | Sel4Arch::Riscv64 => Arch::Riscv,
        }
    }

    /// Create an Arch from the first part of a rust target triple
    pub fn from_rust_arch(rust_arch: RustArch) -> Option<Arch> {
        Sel4Arch::from_rust_arch(rust_arch).map(Arch::from_sel4_arch)
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Arch::Arm => "arm",
            Arch::X86 => "x86",
            Arch::Riscv => "riscv",
        };
        write!(f, "{}", s)
    }
}

/// This is sel4's platform, which we pass around in SEL4_PLATFORM
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Platform(pub String);
impl Display for Platform {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(&mut f)
    }
}

pub(crate) mod raw {
    use super::full::{PlatformBuild, PlatformBuildProfile};
    use super::*;

    pub(crate) struct Raw {
        pub(crate) sel4: SeL4,
        pub(crate) build: Option<BTreeMap<String, PlatformBuild>>,
        pub(crate) metadata: BTreeMap<String, TomlValue>,
    }

    pub(crate) struct SeL4 {
        pub(crate) kernel: TomlTable,
        pub(crate) tools: TomlTable,
        pub(crate) util_libs: TomlTable,
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
                Ok(SeL4 {
                    kernel,
                    tools,
                    util_libs,
                    config,
                })
            }

            fn parse_required_table(
                parent: &TomlTable,
                key: &str,
            ) -> Result<TomlTable, ImportError> {
                if let Some(val) = parent.get(key) {
                    Ok(val.as_table().map(|s| s.to_owned()).ok_or_else(|| {
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
                || ImportError::MissingProperty {
                    name: "sel4".to_string(),
                    expected_type: "table",
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
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Hash)]
pub enum SingleValue {
    String(String),
    Integer(i64),
    Boolean(bool),
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

    pub fn to_toml(&self) -> TomlValue {
        match self {
            SingleValue::String(s) => TomlValue::String(s.clone()),
            SingleValue::Integer(i) => TomlValue::Integer(*i),
            SingleValue::Boolean(b) => TomlValue::Boolean(*b),
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

    fn toml_pair((k, v): (&String, &SingleValue)) -> (String, TomlValue) {
        (k.to_owned(), v.to_toml())
    }
}

fn parse_required_string(table: &TomlTable, key: &str) -> Result<String, ImportError> {
    if let Some(val) = table.get(key) {
        Ok(val
            .as_str()
            .map(|s| s.to_owned())
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

fn parse_optional_string(table: &TomlTable, key: &str) -> Result<Option<String>, ImportError> {
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SeL4Sources {
    pub kernel: RepoSource,
    pub tools: RepoSource,
    pub util_libs: RepoSource,
}

impl SeL4Sources {
    fn relative_to(&self, base_dir: Option<&Path>) -> Self {
        SeL4Sources {
            kernel: self.kernel.relative_to(base_dir),
            tools: self.tools.relative_to(base_dir),
            util_libs: self.util_libs.relative_to(base_dir),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RepoSource {
    LocalPath(PathBuf),
    RemoteGit { url: String, target: GitTarget },
}

impl RepoSource {
    fn relative_to(&self, base_dir: Option<&Path>) -> Self {
        match self {
            RepoSource::LocalPath(p) => RepoSource::LocalPath(p.relative_to(base_dir)),
            s @ _ => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GitTarget {
    Branch(String),
    Rev(String),
    Tag(String),
}

impl GitTarget {
    pub fn kind(&self) -> &'static str {
        match self {
            GitTarget::Branch(_) => "branch",
            GitTarget::Rev(_) => "rev",
            GitTarget::Tag(_) => "tag",
        }
    }
    pub fn value(&self) -> &str {
        match self {
            GitTarget::Branch(s) | GitTarget::Rev(s) | GitTarget::Tag(s) => s,
        }
    }
}

pub mod full {
    use super::*;
    use std::collections::btree_map::BTreeMap;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Full {
        pub sel4: SeL4,
        pub build: BTreeMap<String, PlatformBuild>,
        pub metadata: Metadata,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SeL4 {
        pub sources: SeL4Sources,
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
        pub fn new(sources: SeL4Sources, config: Config) -> Self {
            SeL4 { sources, config }
        }
    }

    pub type Config = PropertiesTree;
    pub type Metadata = PropertiesTree;

    /// A repeated structure that includes common/shared properties,
    /// two optional debug and release sets of properties
    /// and a named bag of bags of properties.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct PropertiesTree {
        pub shared: BTreeMap<String, SingleValue>,
        pub debug: BTreeMap<String, SingleValue>,
        pub release: BTreeMap<String, SingleValue>,
        pub contextual: BTreeMap<String, BTreeMap<String, SingleValue>>,
    }

    impl std::str::FromStr for Full {
        type Err = ImportError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let raw::Raw {
                sel4,
                build,
                metadata,
            } = s.parse()?;
            let sources = SeL4Sources {
                kernel: parse_repo_source(&sel4.kernel)?,
                tools: parse_repo_source(&sel4.tools)?,
                util_libs: parse_repo_source(&sel4.util_libs)?,
            };

            Ok(Full {
                sel4: SeL4 {
                    sources,
                    config: structure_property_tree(sel4.config)?,
                },
                build: build.unwrap_or_else(|| BTreeMap::new()),
                metadata: structure_property_tree(metadata)?,
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
            let mut sel4 = serialize_sel4_sources(&self.sel4.sources);

            fn serialize_sel4_sources(sources: &SeL4Sources) -> TomlTable {
                let mut table = TomlTable::new();
                table.insert_table("kernel", serialize_repo_source(&sources.kernel));
                table.insert_table("tools", serialize_repo_source(&sources.tools));
                table.insert_table("util_libs", serialize_repo_source(&sources.util_libs));
                table
            }
            fn serialize_repo_source(source: &RepoSource) -> TomlTable {
                let mut table = TomlTable::new();
                match source {
                    RepoSource::LocalPath(p) => {
                        table.insert_str("path", format!("{}", p.display()));
                    }
                    RepoSource::RemoteGit { url, target } => {
                        table.insert_str("git", url.as_ref());
                        match target {
                            GitTarget::Branch(v) => table.insert_str("branch", v.as_ref()),
                            GitTarget::Tag(v) => table.insert_str("tag", v.as_ref()),
                            GitTarget::Rev(v) => table.insert_str("rev", v.as_ref()),
                        };
                    }
                }

                table
            }

            fn serialize_properties_tree(source: &PropertiesTree) -> TomlTable {
                let mut properties = TomlTable::new();
                properties.extend(source.shared.iter().map(SingleValue::toml_pair));
                if !source.debug.is_empty() {
                    properties.insert_table(
                        "debug",
                        source.debug.iter().map(SingleValue::toml_pair).collect(),
                    );
                }
                if !source.release.is_empty() {
                    properties.insert_table(
                        "release",
                        source.release.iter().map(SingleValue::toml_pair).collect(),
                    );
                }
                for (k, t) in source.contextual.iter() {
                    properties
                        .insert_table(k.as_ref(), t.iter().map(SingleValue::toml_pair).collect());
                }
                properties
            }
            sel4.insert_table("config", serialize_properties_tree(&self.sel4.config));

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
            let metadata = serialize_properties_tree(&self.metadata);
            if !metadata.is_empty() {
                top.insert_table("metadata", metadata);
            }
            top
        }

        /// Serialize the full contents to a toml string
        pub fn to_toml_string(&self) -> Result<String, TomlSerError> {
            to_string_pretty(&self.to_toml())
        }
    }

    fn toml_table_to_map_of_singles(
        t: &toml::value::Table,
    ) -> Result<BTreeMap<String, SingleValue>, ImportError> {
        t.into_iter().map(SingleValue::single_pair).collect()
    }

    fn structure_property_tree(
        rc: BTreeMap<String, TomlValue>,
    ) -> Result<PropertiesTree, ImportError> {
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

        Ok(PropertiesTree {
            shared,
            debug: debug.unwrap_or_else(BTreeMap::new),
            release: release.unwrap_or_else(BTreeMap::new),
            contextual,
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
        pub sel4_sources: SeL4Sources,
        pub context: Context,
        pub sel4_config: BTreeMap<String, SingleValue>,
        pub build: Build,
        pub metadata: BTreeMap<String, SingleValue>,
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
        pub platform: Platform,
        pub is_debug: bool,
        pub base_dir: Option<PathBuf>,
        pub arch: Arch,
        pub sel4_arch: Sel4Arch,
    }

    impl Contextualized {
        pub fn from_str(
            source_toml: &str,
            arch: Arch,
            sel4_arch: Sel4Arch,
            is_debug: bool,
            platform: Platform,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let f: full::Full = source_toml.parse()?;
            Self::from_full(f, arch, sel4_arch, is_debug, platform, base_dir)
        }

        pub fn from_full(
            mut f: full::Full,
            arch: Arch,
            sel4_arch: Sel4Arch,
            is_debug: bool,
            platform: Platform,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let context = Context {
                platform: platform.clone(),
                arch,
                sel4_arch,
                is_debug,
                base_dir: base_dir.map(|p| p.to_path_buf()),
            };

            let platform_build = f.build.remove(&platform.to_string()).ok_or_else(|| {
                ImportError::NoBuildSupplied {
                    platform: platform.to_string(),
                    profile: if is_debug { "debug" } else { "release " },
                }
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

            fn resolve_context(
                tree: &full::PropertiesTree,
                context: &Context,
            ) -> BTreeMap<String, SingleValue> {
                let mut flat_properties = tree.shared.clone();
                if context.is_debug {
                    flat_properties.extend(tree.debug.clone())
                } else {
                    flat_properties.extend(tree.release.clone())
                }

                if let Some(arch_props) = tree.contextual.get(&context.arch.to_string()) {
                    flat_properties.extend(arch_props.clone());
                }
                if let Some(sel4_arch_props) = tree.contextual.get(&context.sel4_arch.to_string()) {
                    flat_properties.extend(sel4_arch_props.clone());
                }
                if let Some(platform_props) = tree.contextual.get(&context.platform.to_string()) {
                    flat_properties.extend(platform_props.clone());
                }
                flat_properties
            }

            let sel4_config = resolve_context(&f.sel4.config, &context);
            let metadata = resolve_context(&f.metadata, &context);

            let sources = f.sel4.sources.relative_to(base_dir);

            Ok(Contextualized {
                sel4_sources: sources,
                context,
                sel4_config,
                build,
                metadata,
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
                    sources: SeL4Sources {
                        kernel: RepoSource::LocalPath(PathBuf::from(".")),
                        tools: RepoSource::LocalPath(PathBuf::from(".")),
                        util_libs: RepoSource::LocalPath(PathBuf::from(".")),
                    },
                    config: Default::default(),
                },
                build: Default::default(),
                metadata: Default::default(),
            }
        }
    }

    #[test]
    fn default_content_is_valid() {
        let f: full::Full = get_default_config();
        assert_eq!(
            RepoSource::RemoteGit {
                url: "https://github.com/seL4/seL4".to_string(),
                target: GitTarget::Tag("10.1.1".to_string())
            },
            f.sel4.sources.kernel
        )
    }

    #[test]
    fn override_default_platform_contextualization() {
        let mut f = full::Full::empty();
        let expected = Platform("sabre".to_owned());
        f.build.insert(
            expected.to_string(),
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
            Arch::Arm,
            Sel4Arch::Aarch32,
            false,
            expected.clone(),
            None,
        )
        .unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(false, c.context.is_debug);
        assert_eq!(Arch::Arm, c.context.arch);
        assert_eq!(Sel4Arch::Aarch32, c.context.sel4_arch);
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
