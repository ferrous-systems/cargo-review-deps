#[macro_use]
extern crate failure;
extern crate cargo_metadata;
extern crate copy_dir;
extern crate semver;
extern crate tempdir;

use std::{
    fmt, fs,
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use copy_dir::copy_dir;
use semver::Version;
use tempdir::TempDir;

pub use failure::Error;
pub type Result<T> = ::std::result::Result<T, Error>;

/// Mirrors `PackageId` from Cargo. `PackageId` is an unambiguous reference to a
/// package version.
///
/// Future work: support git dependencies and alternative registries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId {
    name: String,
    version: Version,
}

impl fmt::Display for PackageId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.name.fmt(fmt)?;
        fmt.write_str(":")?;
        self.version.fmt(fmt)
    }
}

impl FromStr for PackageId {
    type Err = Error;
    fn from_str(s: &str) -> Result<PackageId> {
        let colon_idx = s.find(':').ok_or_else(|| {
            format_err!(
                "invalid package specification: {:?}; expected \"name:x.y.z\"",
                s
            )
        })?;
        let name = s[..colon_idx].to_string();
        let version: Version = s[colon_idx + 1..].parse()?;
        Ok(PackageId { name, version })
    }
}

#[derive(Debug)]
pub struct Diff {
    pub first: PackageId,
    pub second: PackageId,
    pub dest: Option<PathBuf>,
}

impl Diff {
    pub fn run(self) -> Result<()> {
        let first_src = fetch(&self.first)?;
        let second_src = fetch(&self.second)?;
        if let Some(dir) = self.dest {
            fs::create_dir_all(&dir)?;
            copy_dir(&first_src, &dir.join(self.first.to_string()))?;
            copy_dir(&second_src, &dir.join(self.second.to_string()))?;
        } else {
            let mut diff_cmd = Command::new("diff");
            diff_cmd
                .arg("-r")
                .arg(&first_src)
                .arg(&second_src)
                .stdout(Stdio::inherit())
                .status()?;
        }
        Ok(())
    }
}

/// Shells out to Cargo to download `pkg_id` from crates io.
/// Returns the directory with the downloaded package;
fn fetch(pkg_id: &PackageId) -> Result<PathBuf> {
    let dir = TempDir::new("cargo-diff-fetches")?;
    let temp_manifest = dir.path().join("Cargo.toml");
    fs::write(&temp_manifest, format_cargo_toml(pkg_id))?;

    let metadata = cargo_metadata::metadata_deps(
        Some(&temp_manifest),
        true, // include dependencies
    ).map_err(|err| format_err!("cargo metadata failed: {}", err))?; // error_chain is not sync :-(

    let package = metadata
        .packages
        .iter()
        .find(|it| it.name == pkg_id.name && it.version == pkg_id.version.to_string())
        .ok_or_else(|| format_err!("unexpected error: can't find package {:?}", pkg_id))?;

    let res = PathBuf::from(&package.manifest_path)
        .parent()
        .ok_or_else(|| {
            format_err!(
                "unexpected error: bad manifest path {:?}",
                package.manifest_path
            )
        })?.to_path_buf();
    Ok(res)
}

/// Conjures up a Cargo.toml with `pkg_id` as a dependency.
fn format_cargo_toml(pkg_id: &PackageId) -> String {
    format!(
        r#"
[package]
name = "cargo-diff-temp-pkg"
version = "0.0.0"

[lib]
path = "./Cargo.toml"

[dependencies]
{} = "={}"
"#,
        pkg_id.name, pkg_id.version
    )
}
