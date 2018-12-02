extern crate cargo_diff;
extern crate structopt;

use std::path::PathBuf;

use cargo_diff::{Diff, PackageId, Result};
use structopt::StructOpt;

/// Compares two published versions of a crate.
#[derive(StructOpt, Debug)]
#[structopt(name = "cargo-diff")]
struct Opt {
    /// First package to compare, in the form of name:version, for example
    /// regex:1.0.0.
    #[structopt(name = "FIRST_PACKAGE_ID")]
    first: PackageId,
    /// Second package to compare, in the form of name:version, for example
    /// regex:1.1.2.
    #[structopt(name = "SECOND_PACKAGE_ID")]
    second: PackageId,
    /// If specified, will copy the sources of the packages to the specified dir
    /// instead of running the diff command.
    #[structopt(long = "copy-to", name = "DIR", parse(from_os_str))]
    copy_to: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let diff = Diff {
        first: opt.first,
        second: opt.second,
        copy_to: opt.copy_to,
    };
    diff.run()?;
    Ok(())
}
