extern crate cargo_review_deps;
extern crate clap;

use std::{ffi::OsStr, path::PathBuf};

use cargo_review_deps::{Current, Diff, PackageId, Result, UpdateDiff};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

fn main() {
    let exit_code = main_inner();
    std::process::exit(exit_code);
}

fn main_inner() -> i32 {
    let matches = App::new("cargo-review-deps")
        .bin_name("cargo")
        .version("1.0")
        .settings(&[AppSettings::GlobalVersion, AppSettings::SubcommandRequired])
        .subcommand(
            SubCommand::with_name("review-deps")
                .author("Aleksey Kladov <aleksey.kladov@ferrous-systems.com>")
                .about("Helps you to review source code of your crates.io dependencies")
                .setting(AppSettings::SubcommandRequired)
                .subcommand(
                    SubCommand::with_name("diff")
                        .about("Show the diff between two crate versions")
                        .after_help("By default, diff -r command is used for diffing. \
                               If you want to use a custom diff tool, specify the --destination \
                               argument and run the diff command manually.")
                        .arg(
                            Arg::with_name("FIRST_PACKAGE_ID")
                                .required(true)
                                .index(1)
                                .help("First crate to diff, in the form of name:version, for example rand:0.6.0"),
                        )
                        .arg(
                            Arg::with_name("SECOND_PACKAGE_ID")
                                .required(true)
                                .index(2)
                                .help("Second crate to diff, for example rand:0.6.1"),
                        )
                        .arg(
                            Arg::with_name("destination")
                                .short("d")
                                .long("destination")
                                .takes_value(true)
                                .value_name("DIR")
                                .help("Checkout sources of the two versions to the specified directory")
                        ),
                )
                .subcommand(
                    SubCommand::with_name("current")
                        .about("Show the diff for dependencies after cargo update")
                        .after_help("By default, diff -r command is used for diffing. \
                               If you want to use a custom diff tool, specify the --destination \
                               argument and run the diff command manually.")
                        .arg(
                            Arg::with_name("destination")
                                .short("d")
                                .long("destination")
                                .takes_value(true)
                                .value_name("DIR")
                                .required(true)
                                .help("Checkout sources of the two versions to the specified directory")
                        ),
                )
                .subcommand(
                    SubCommand::with_name("update-diff")
                        .about("Download source code of dependencies which would be changed by cargo update to the specified directory")
                        .after_help("By default, diff -r command is used for diffing. \
                               If you want to use a custom diff tool, specify the --destination \
                               argument and run the diff command manually.")
                       .arg(
                            Arg::with_name("destination")
                                .short("d")
                                .long("destination")
                                .takes_value(true)
                                .value_name("DIR")
                                .help("Checkout sources of dependencies to the specified directory")
                        )
                        .arg(
                            Arg::with_name("args")
                                .last(true)
                                .multiple(true)
                        )
                ),
        ).get_matches();

    let matches = matches.subcommand_matches("review-deps").unwrap(); // Cargo always calls us using `cargo review-deps ...` as `argv`
    let (cmd, matches) = match matches.subcommand() {
        (cmd, Some(matches)) => (cmd, matches),
        (_, None) => unreachable!("AppSettings::SubcommandRequired is set"),
    };

    let res = match cmd {
        "diff" => exec_diff(&matches),
        "current" => exec_current(&matches),
        "update-diff" => exec_update_diff(&matches),
        _ => unreachable!("no such cmd: {:?}", cmd),
    };

    if let Err(err) = res {
        eprintln!("error: {}", err);
        return 101;
    }
    0
}

fn value_of_pkg_id(matches: &ArgMatches, arg_name: &str) -> Result<PackageId> {
    let value = matches.value_of(arg_name).expect("arg is required");
    value.parse()
}

fn exec_diff(matches: &ArgMatches) -> Result<()> {
    let first = value_of_pkg_id(&matches, "FIRST_PACKAGE_ID")?;
    let second = value_of_pkg_id(&matches, "SECOND_PACKAGE_ID")?;
    let dest = matches.value_of("destination").map(PathBuf::from);
    Diff {
        first,
        second,
        dest,
    }
    .run()
}

fn exec_current(matches: &ArgMatches) -> Result<()> {
    let dest = matches.value_of("destination").unwrap().into();
    Current { dest }.run()
}

fn exec_update_diff(matches: &ArgMatches) -> Result<()> {
    let dest = matches.value_of("destination").map(PathBuf::from);
    let args = matches
        .values_of_os("args")
        .unwrap_or_default()
        .map(OsStr::to_owned)
        .collect();
    UpdateDiff { dest, args }.run()
}
