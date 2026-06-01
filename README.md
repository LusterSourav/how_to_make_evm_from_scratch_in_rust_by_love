# cargo-registry

This branch is the **Cargo registry index** for `bare-metal-evm-*`.

It is consumed by the `github` registry defined in `.cargo/config.toml`
on `main` (and other branches). Do not edit by hand except for the
initial setup; `cargo publish --registry github` will push index
entries here as new commits on `publish/<crate>-<version>` branches,
which a maintainer then merges into this branch.

## Layout

- `config.json` -- registry config; the `dl` URL points at the
  GitHub Releases page of this repo. `.crate` tarballs must be
  uploaded there as release assets named `<crate>-<version>.crate`.
- `<two>/<two>/<name>` -- one file per published crate, containing
  a JSON line per version. Path prefix is derived from the crate
  name (e.g. `ke/cc/keccak`, `ba/re/-m/et` for `bare-metal-evm-types`
  in the legacy index format; or one of the 1-/2-/3-char variants
  Cargo picks).

## Consuming

Add to a downstream project's `.cargo/config.toml`:

    [source.crates-io]
    replace-with = "github"

Or depend on a specific crate with:

    bare-metal-evm-keccak = { version = "0.2", registry = "github" }

## Publishing

See `RELEASE.md` on `main`.
