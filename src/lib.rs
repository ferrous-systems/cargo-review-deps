#[macro_use]
extern crate failure;
extern crate cargo_metadata;
extern crate copy_dir;
extern crate semver;
extern crate tempdir;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
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
            let diff_status = diff_cmd
                .args(&["--color=auto", "-r"])
                .arg(&first_src)
                .arg(&second_src)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();
            if diff_status.is_err() {
                if !has_diff_cmd() {
                    bail!("looks like you don't have a suitable diff command installed.\n\
                           Try using --destination flag to run a custom diff tool or to compare sources manually.")
                }
            }
            diff_status?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Current {
    pub dest: PathBuf,
}

impl Current {
    pub fn run(self) -> Result<()> {
        let metadata = Metadata {
            manifest_path: None,
        }.run()?;

        fs::create_dir_all(&self.dest)?;
        for pkg in metadata.packages.iter() {
            // Ideally we should look at the `source`, but that is private.
            let is_cratesio_dep = pkg.id.contains("crates.io-index");
            if !is_cratesio_dep {
                eprintln!(
                    "Skipping package `{}`: not a crates.io dependency",
                    pkg.name
                );
                continue;
            }
            let src = pkg_dir(&pkg)?;
            let dst = self.dest.join(format!("{}:{}", pkg.name, pkg.version));
            copy_dir(&src, &dst)?;
        }
        Ok(())
    }
}

struct Metadata<'a> {
    manifest_path: Option<&'a Path>,
}

impl<'a> Metadata<'a> {
    fn run(self) -> Result<cargo_metadata::Metadata> {
        let metadata = cargo_metadata::metadata_deps(
            self.manifest_path,
            true, // include dependencies
        ).map_err(|err| format_err!("cargo metadata failed: {}", err))?; // error_chain is not sync :-(
        Ok(metadata)
    }
}

fn has_diff_cmd() -> bool {
    match Command::new("diff").arg("--version").status() {
        Err(_) => false,
        Ok(status) => status.success(),
    }
}

/// Shells out to Cargo to download `pkg_id` from crates io.
/// Returns the directory with the downloaded package;
fn fetch(pkg_id: &PackageId) -> Result<PathBuf> {
    let dir = TempDir::new("cargo-diff-fetches")?;
    let temp_manifest = dir.path().join("Cargo.toml");
    fs::write(&temp_manifest, format_cargo_toml(pkg_id))?;
    let metadata = Metadata {
        manifest_path: Some(temp_manifest.as_path()),
    }.run()?;

    let package = metadata
        .packages
        .iter()
        .find(|it| it.name == pkg_id.name && it.version == pkg_id.version.to_string())
        .ok_or_else(|| format_err!("unexpected error: can't find package {:?}", pkg_id))?;
    pkg_dir(&package)
}

fn pkg_dir(pkg: &cargo_metadata::Package) -> Result<PathBuf> {
    let res = PathBuf::from(&pkg.manifest_path)
        .parent()
        .ok_or_else(|| {
            format_err!(
                "unexpected error: bad manifest path {:?}",
                pkg.manifest_path
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
