use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod deserialization;
pub mod serialization;

pub use deserialization::ImportError;

const DEFAULT_CONFIG_CONTENT: &str = include_str!("../default_config.toml");

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
pub enum SeL4Arch {
    Aarch32,
    Aarch64,
    ArmHyp,
    Ia32,
    X86_64,
    Riscv32,
    Riscv64,
}

impl FromStr for SeL4Arch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch32" => Ok(SeL4Arch::Aarch32),
            "aarch64" => Ok(SeL4Arch::Aarch64),
            "arm_hyp" => Ok(SeL4Arch::ArmHyp),
            "ia32" => Ok(SeL4Arch::Ia32),
            "riscv32" => Ok(SeL4Arch::Riscv32),
            "riscv64" => Ok(SeL4Arch::Riscv64),
            "x86_64" => Ok(SeL4Arch::X86_64),
            _ => Err("Unrecognized sel4_arch".to_string()),
        }
    }
}

impl SeL4Arch {
    /// Create an Arch from the first part of a rust target triple
    pub fn from_rust_arch(rust_arch: RustArch) -> Option<SeL4Arch> {
        match rust_arch {
            RustArch::Aarch64 => Some(SeL4Arch::Aarch64),

            RustArch::Arm
            | RustArch::Armebv7r
            | RustArch::Armv7
            | RustArch::Armv7r
            | RustArch::Armv7s => Some(SeL4Arch::Aarch32),

            RustArch::I386 | RustArch::I586 | RustArch::I686 => Some(SeL4Arch::Ia32),

            RustArch::Riscv32imac | RustArch::Riscv32imc => Some(SeL4Arch::Riscv32),

            RustArch::Riscv64gc | RustArch::Riscv64imac => Some(SeL4Arch::Riscv64),

            RustArch::Thumbv6m
            | RustArch::Thumbv7em
            | RustArch::Thumbv7m
            | RustArch::Thumbv7neon => Some(SeL4Arch::Aarch32),

            RustArch::Thumbv8mmain => Some(SeL4Arch::Aarch64),

            RustArch::X86_64 => Some(SeL4Arch::X86_64),
            _ => None,
        }
    }
}

impl Display for SeL4Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SeL4Arch::Aarch32 => "aarch32",
            SeL4Arch::Aarch64 => "aarch64",
            SeL4Arch::ArmHyp => "arm_hyp",
            SeL4Arch::Ia32 => "ia32",
            SeL4Arch::X86_64 => "x86_64",
            SeL4Arch::Riscv32 => "riscv32",
            SeL4Arch::Riscv64 => "riscv64",
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
    pub fn from_sel4_arch(sel4_arch: SeL4Arch) -> Arch {
        match sel4_arch {
            SeL4Arch::Aarch32 | SeL4Arch::Aarch64 | SeL4Arch::ArmHyp => Arch::Arm,
            SeL4Arch::Ia32 | SeL4Arch::X86_64 => Arch::X86,
            SeL4Arch::Riscv32 | SeL4Arch::Riscv64 => Arch::Riscv,
        }
    }

    /// Create an Arch from the first part of a rust target triple
    pub fn from_rust_arch(rust_arch: RustArch) -> Option<Arch> {
        SeL4Arch::from_rust_arch(rust_arch).map(Arch::from_sel4_arch)
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Hash)]
pub enum SingleValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SeL4Sources {
    pub kernel: RepoSource,
    pub tools: RepoSource,
    pub util_libs: RepoSource,
}

impl SeL4Sources {
    fn relative_to<P: AsRef<Path>>(&self, base_dir: &Option<P>) -> Self {
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
    fn relative_to<P: AsRef<Path>>(&self, base_dir: &Option<P>) -> Self {
        match self {
            RepoSource::LocalPath(p) => RepoSource::LocalPath(p.relative_to(base_dir)),
            s => s.clone(),
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
	pub build_dir: Option<PathBuf>,
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
        pub fn new(sources: SeL4Sources, build_dir: Option<PathBuf>, config: Config) -> Self {
            SeL4 { sources, build_dir, config }
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
}

trait RelativePath {
    // If self is relative, and a base is supplied, evaluate self relative to
    // the base. Otherwise hand back self.
    fn relative_to<P: AsRef<Path>>(&self, base: &Option<P>) -> PathBuf;
}

impl RelativePath for Path {
    fn relative_to<P: AsRef<Path>>(&self, base: &Option<P>) -> PathBuf {
        if self.is_relative() {
            match base {
                Some(p) => p.as_ref().join(self),
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
	pub build_dir: Option<PathBuf>,
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
        pub sel4_arch: SeL4Arch,
    }

    impl Contextualized {
        pub fn from_str(
            source_toml: &str,
            arch: Arch,
            sel4_arch: SeL4Arch,
            is_debug: bool,
            platform: Platform,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let f: full::Full = source_toml.parse()?;
            Self::from_full(&f, arch, sel4_arch, is_debug, platform, base_dir)
        }

        pub fn from_full(
            f: &full::Full,
            arch: Arch,
            sel4_arch: SeL4Arch,
            is_debug: bool,
            platform: Platform,
            base_dir: Option<&Path>,
        ) -> Result<Contextualized, ImportError> {
            let context = Context {
                platform,
                arch,
                sel4_arch,
                is_debug,
                base_dir: base_dir.map(Path::to_path_buf),
            };
            Contextualized::from_full_context(f, context)
        }

        pub fn from_full_context(
            f: &full::Full,
            context: Context,
        ) -> Result<Contextualized, ImportError> {
            let platform_build = f
                .build
                .get(&context.platform.to_string())
                .ok_or_else(|| ImportError::NoBuildSupplied {
                    platform: context.platform.to_string(),
                    profile: if context.is_debug {
                        "debug"
                    } else {
                        "release "
                    },
                })?
                .clone();
            let build_profile = if context.is_debug {
                platform_build.debug_build_profile
            } else {
                platform_build.release_build_profile
            };
            let root_task = build_profile.map(|bp| RootTask {
                make_command: bp.make_root_task,
                image_path: bp.root_task_image.relative_to(&context.base_dir),
            });
            let build = Build {
                cross_compiler_prefix: platform_build.cross_compiler_prefix,
                toolchain_dir: platform_build
                    .toolchain_dir
                    .map(|p| p.relative_to(&context.base_dir)),
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

            let sel4_sources = f.sel4.sources.relative_to(&context.base_dir);
	    let build_dir = f.sel4.build_dir.clone();

            Ok(Contextualized {
                sel4_sources,
		build_dir,
                context,
                sel4_config,
                build,
                metadata,
            })
        }
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
		    build_dir: None,
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
        // Spot check a known piece of the default config content
        assert_eq!(
            RepoSource::RemoteGit {
                url: "https://github.com/seL4/seL4".to_string(),
                target: GitTarget::Rev("4d0f02c029560cae0e8d93727eb17d58bcecc2ac".to_string())
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
            &f,
            Arch::Arm,
            SeL4Arch::Aarch32,
            false,
            expected.clone(),
            None,
        )
        .unwrap();
        assert_eq!(expected, c.context.platform);
        assert_eq!(false, c.context.is_debug);
        assert_eq!(Arch::Arm, c.context.arch);
        assert_eq!(SeL4Arch::Aarch32, c.context.sel4_arch);
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
