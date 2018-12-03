# cargo-review-deps

A cargo subcommand for reviewing the source code of crates.io dependencies.

## Installation:

```
cargo install cargo-review-deps
```

## Usage:

To see the `diff -r` of packages, use

```
$ cargo review-deps diff rand:0.6.0 rand:0.6.1
```

If you want to use a custom diff tool or need to do a more thorough
investigation, use `--destination` option to checkout sources of dependencies
locally.

```
$ cargo review-deps diff rand:0.6.0 rand:0.6.1 --destinations diff
```

The `diff/random:0.6.0` and `diff/random:0.6.1` directories would
contain the sources of the respective versions.

Note that `cargo-review-deps` does not rely on version control information: it
uses exactly that version of source code, that will be used by Cargo to build
your project.

## Future plans:

`cargo review-deps current -d my-deps` to dump the sources of all dependencies
to `my-deps` directory.

`cargo review-deps diff-update -d diff -- -p rand --precise 0.6.1` to get the
diff of *all* dependencies changed during `cargo update`.

## Similar projects:

[cargo-audit](https://github.com/RustSec/cargo-audit) checks your project for
dependencies with security vulnerabilities reported to the RustSec Advisory
Database.
