# Cargo diff

Utility for showing diff between versions of crates.io packages.

Usage:

To see the `diff -r` of packages, use

```
$ cargo-diff rand:0.6.0 rand:0.6.1
```

To checkout sources of packages side by side, to do a more thorough
comparison, use

```
$ cargo-diff rand:0.6.0 rand:0.6.1 --copy-to diff
```

The `diff/random:0.6.0` and `diff/random:0.6.1` directories would
contain the sources of the respective versions.
