#[macro_use]
extern crate failure;
extern crate cargo_metadata;
extern crate copy_dir;
extern crate semver;
extern crate tempdir;

use std::{
    collections::HashMap,
    ffi::OsString,
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
            run_diff_cmd(&first_src, &second_src)?;
        }
        Ok(())
    }
}

pub fn run_diff_cmd(a: &Path, b: &Path) -> Result<()> {
    let mut diff_cmd = Command::new("diff");
    let diff_status = diff_cmd
        .args(&["--color=auto", "-r"])
        .arg(a)
        .arg(b)
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
    Ok(())
}

#[derive(Debug)]
pub struct Current {
    pub dest: PathBuf,
}

impl Current {
    pub fn run(self) -> Result<()> {
        let metadata = Metadata {
            manifest_path: None,
        }
        .run()?;

        fs::create_dir_all(&self.dest)?;
        for pkg in crates_io_packages(&metadata) {
            let src = pkg_dir(&pkg)?;
            let dst = self.dest.join(format!("{}:{}", pkg.name, pkg.version));
            copy_dir(&src, &dst)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateDiff {
    pub dest: Option<PathBuf>,
    pub args: Vec<OsString>,
}

impl UpdateDiff {
    pub fn run(self) -> Result<()> {
        let before_metadata = Metadata {
            manifest_path: None,
        }
        .run()?;
        let workspace_root = Path::new(&before_metadata.workspace_root);
        let lockfile = workspace_root.join("Cargo.lock");
        let mut lockfile_guard = LockfileGuard::new(lockfile)?;

        let status = Command::new("cargo")
            .arg("update")
            .args(&self.args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            bail!("running cargo update failed");
        }
        let after_metadata = Metadata {
            manifest_path: None,
        }
        .run()?;
        let tmpdir;
        let dest = match self.dest.as_ref() {
            Some(it) => it.as_path(),
            None => {
                tmpdir = TempDir::new("cargo-diff-fetches")?;
                tmpdir.path()
            }
        };
        let before_dir = dest.join("before");
        let after_dir = dest.join("after");
        fs::create_dir_all(&before_dir)?;
        fs::create_dir_all(&after_dir)?;
        for pdiff in metadata_diff(&before_metadata, &after_metadata)? {
            pdiff.dump_to(&dest)?;
        }

        if self.dest.is_none() {
            run_diff_cmd(&before_dir, &after_dir)?
        }
        lockfile_guard.restore_lockfile()?;
        Ok(())
    }
}

#[derive(Debug)]
struct PackageDiff {
    name: String,
    before: Option<PathBuf>,
    after: Option<PathBuf>,
}

impl PackageDiff {
    fn dump_to(&self, dest: &Path) -> Result<()> {
        if let Some(src) = self.before.as_ref() {
            let dst = dest.join("before").join(&self.name);
            copy_dir(&src, &dst)?;
        }
        if let Some(src) = self.after.as_ref() {
            let dst = dest.join("after").join(&self.name);
            copy_dir(&src, &dst)?;
        }
        Ok(())
    }
}

fn metadata_diff(
    before: &cargo_metadata::Metadata,
    after: &cargo_metadata::Metadata,
) -> Result<Vec<PackageDiff>> {
    let before = extract_packages(before)?;
    let after = extract_packages(after)?;
    let mut res = Vec::new();
    for (name, before_path) in before.iter() {
        if !after.contains_key(name) {
            res.push(PackageDiff {
                name: name.clone(),
                before: Some(before_path.clone()),
                after: None,
            })
        }
    }
    for (name, after_path) in after.iter() {
        if !before.contains_key(name) {
            res.push(PackageDiff {
                name: name.clone(),
                before: None,
                after: Some(after_path.clone()),
            })
        }
    }
    for (name, before_path) in before.iter() {
        if let Some(after_path) = after.get(name) {
            if before_path == after_path {
                continue;
            }
            res.push(PackageDiff {
                name: name.clone(),
                before: Some(before_path.clone()),
                after: Some(after_path.clone()),
            });
        }
    }

    Ok(res)
}

fn extract_packages(meta: &cargo_metadata::Metadata) -> Result<HashMap<String, PathBuf>> {
    let mut res = HashMap::new();
    for pkg in crates_io_packages(meta) {
        let version = Version::parse(&pkg.version)?;
        let semver_compatible_version = if version.major == 0 {
            format!("0.{}", version.minor)
        } else {
            format!("{}", version.major)
        };
        let name = format!("{}:{}", pkg.name, semver_compatible_version);
        res.insert(name, pkg_dir(pkg)?);
    }
    Ok(res)
}

fn crates_io_packages<'a>(
    meta: &'a cargo_metadata::Metadata,
) -> impl Iterator<Item = &'a cargo_metadata::Package> {
    meta.packages.iter().filter(|pkg| {
        // Ideally we should look at the `source`, but that is private.
        let is_cratesio_dep = pkg.id.contains("crates.io-index");
        if !is_cratesio_dep {
            eprintln!(
                "Skipping package `{}`: not a crates.io dependency",
                pkg.name
            );
        }
        is_cratesio_dep
    })
}

/// We run real `cargo update` which writes to the lockfile. This struct makes sure (in
/// Drop), that we restore it propertly afterwards.
#[derive(Debug)]
struct LockfileGuard {
    lockfile_path: PathBuf,
    lockfile_copy_path: PathBuf,
    lockfile_contents: String,
    restored: bool,
}

impl LockfileGuard {
    fn new(path: impl Into<PathBuf>) -> Result<LockfileGuard> {
        let lockfile_path = path.into();
        let lockfile_copy_path = lockfile_path.with_extension(".lock.back");
        let lockfile_contents = fs::read_to_string(&lockfile_path)?;
        fs::write(&lockfile_copy_path, &lockfile_contents)?;
        let res = LockfileGuard {
            lockfile_path,
            lockfile_copy_path,
            lockfile_contents,
            restored: false,
        };
        Ok(res)
    }

    fn restore_lockfile(&mut self) -> Result<()> {
        self.restored = true;
        fs::write(&self.lockfile_path, &self.lockfile_contents)?;
        fs::remove_file(self.lockfile_copy_path.as_path())?;
        Ok(())
    }
}

impl Drop for LockfileGuard {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore_lockfile();
        }
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
        )
        .map_err(|err| format_err!("cargo metadata failed: {}", err))?; // error_chain is not sync :-(
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
    }
    .run()?;

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
        })?
        .to_path_buf();
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
