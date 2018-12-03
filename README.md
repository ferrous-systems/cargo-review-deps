# cargo-review-deps

A cargo subcommand for reviewing the source code of crates.io dependencies.

Installation:

```
cargo install cargo-review-deps
```

Usage:

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
