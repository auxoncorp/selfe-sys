use super::full;
use super::{GitTarget, RepoSource, SeL4Sources, SingleValue};
use std::collections::BTreeMap;
use toml::ser::{to_string_pretty, Error as TomlSerError};
use toml::value::{Table as TomlTable, Value as TomlValue};

impl full::Full {
    fn to_toml(&self) -> TomlTable {
        let mut sel4 = serialize_sel4_sources(&self.sel4.sources);
        let config = serialize_properties_tree(&self.sel4.config);
        if !config.is_empty() {
            sel4.insert_table("config", config);
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
            table.insert_str("git", url.as_str());
            match target {
                GitTarget::Branch(v) => table.insert_str("branch", v.as_str()),
                GitTarget::Tag(v) => table.insert_str("tag", v.as_str()),
                GitTarget::Rev(v) => table.insert_str("rev", v.as_str()),
            };
        }
    }

    table
}

fn serialize_properties_tree(source: &full::PropertiesTree) -> TomlTable {
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
        properties.insert_table(k.as_str(), t.iter().map(SingleValue::toml_pair).collect());
    }
    properties
}

fn serialize_build(source: &BTreeMap<String, full::PlatformBuild>) -> Option<TomlTable> {
    if source.is_empty() {
        return None;
    }
    let mut build = TomlTable::new();
    for (k, plat) in source.iter() {
        let mut plat_table = TomlTable::new();
        if let Some(ref v) = plat.cross_compiler_prefix {
            plat_table.insert_str("cross_compiler_prefix", v.as_str());
        }
        if let Some(ref v) = plat.toolchain_dir {
            plat_table.insert_str("toolchain_dir", format!("{}", v.display()));
        }

        if let Some(t) = serialize_profile_build(&plat.debug_build_profile) {
            plat_table.insert_table("debug", t);
        }
        if let Some(t) = serialize_profile_build(&plat.release_build_profile) {
            plat_table.insert_table("release", t);
        }
        build.insert_table(k.as_str(), plat_table);
    }
    Some(build)
}

fn serialize_profile_build(source: &Option<full::PlatformBuildProfile>) -> Option<TomlTable> {
    source.as_ref().map(|v| {
        let mut prof_table = TomlTable::new();
        if let Some(mrt) = v.make_root_task.as_ref() {
            prof_table.insert_str("make_root_task", mrt.as_str());
        }
        prof_table.insert_str(
            "root_task_image",
            format!("{}", v.root_task_image.display()),
        );
        prof_table
    })
}

impl SingleValue {
    pub fn to_toml(&self) -> TomlValue {
        match self {
            SingleValue::String(s) => TomlValue::String(s.clone()),
            SingleValue::Integer(i) => TomlValue::Integer(*i),
            SingleValue::Boolean(b) => TomlValue::Boolean(*b),
        }
    }

    fn toml_pair((k, v): (&String, &SingleValue)) -> (String, TomlValue) {
        (k.to_owned(), v.to_toml())
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
