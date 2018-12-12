[![Build Status](https://travis-ci.com/ferrous-systems/cargo-review-deps.svg?branch=master)](https://travis-ci.com/ferrous-systems/cargo-review-deps)

# cargo-review-deps

A cargo subcommand for reviewing the source code of crates.io dependencies.

## Installation:

```
cargo install cargo-review-deps
```

## Usage

### update-diff

To see what exactly changes if you run `cargo-update`, use

```
$ cargo review-deps update-diff -- --package foo
```

This will run (without actually updating the lockfile) `cargo update --package foo`
and show `diff --color -r` of all added/removed/updated dependencies.

If you want to use a custom diff tool or need to do a more thorough
investigation, use `--destination` option to checkout sources of dependencies
locally.

### diff

To quickly see the `diff -r` of two package versions, use

```
$ cargo review-deps diff rand:0.6.0 rand:0.6.1
```

Similarly to `update-diff`, you can use `--destination` option for customized
diffing.

```
$ cargo review-deps diff rand:0.6.0 rand:0.6.1 --destinations diff
```

The `diff/random:0.6.0` and `diff/random:0.6.1` directories would
contain the sources of the respective versions.

Note that `cargo-review-deps` does not rely on version control information: it
uses exactly that version of source code, that will be used by Cargo to build
your project.


### current

To see the sources of all transitive dependencies, use

```
$ cargo review-deps current --destination dir/to/dump/sources/to
```

This will download sources of all of the dependencies to the specified
directory.

## Similar projects:

[cargo-audit](https://github.com/RustSec/cargo-audit) checks your project for
dependencies with security vulnerabilities reported to the RustSec Advisory
Database.

## Commercial Support

This project is developed by [Ferrous Systems GmbH](https://ferrous-systems.com). Interested in commercial support, custom functionality, or sponsoring this open source work? Send us [an email here](mailto:commercial@ferrous-systems.com).

## License

`MIT OR Apache-2.0`
