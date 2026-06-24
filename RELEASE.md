# Release Process

This project publishes to crates.io and to a self-hosted Cargo
registry living in this repo (a `cargo-registry` branch serving
as the index, with `.crate` tarballs attached to GitHub Releases).

## One-time setup

### 1. crates.io token

- Go to <https://crates.io/settings/tokens>
- Create a new token with the `publish` scope
- Copy the token

### 2. Create the `cargo-registry` branch

The self-hosted registry index lives on a dedicated branch. Cargo
clones the branch, reads `config.json` from its root, and pulls
`.crate` files from the `dl` URL declared there.

The branch is created locally during development. To push it to
origin (required before the registry is usable):

```bash
git worktree list                                   # see worktrees
git push origin cargo-registry                      # one-time push
git worktree remove /tmp/cargo-registry-worktree    # clean up local worktree
```

After this push, the `github` registry in `.cargo/config.toml`
will be reachable and `cargo publish --dry-run --registry github`
will work end-to-end.

### 3. GitHub token for the `publish` CI job

- Go to GitHub -> Settings -> Developer settings -> Personal access tokens
- Create a fine-grained token scoped to this repo with `contents: write`
  (so the CI job can push the `publish/<crate>-<version>` branches
  that Cargo creates on the index repo) and `packages: write` (for
  the GitHub Packages release artifacts, if you decide to also push
  there)
- Copy the token

### 4. Add secrets to GitHub

- Repo -> Settings -> Secrets and variables -> Actions -> New repository secret
- Name: `CARGO_TOKEN`, value: the crates.io token
- Name: `CARGO_PUBLISH_TOKEN`, value: a GitHub PAT (fine-grained, see step 3)

The CI `publish` job uses `CARGO_TOKEN` for the crates.io publish
and `CARGO_PUBLISH_TOKEN` for the git-based registry step.

## Cutting a release

```bash
# Make sure everything is green locally first
cargo test --workspace
cargo clippy --workspace --all-features -- -D warnings

# Bump the version in all 7 Cargo.toml files plus the root Cargo.toml
# Update CHANGELOG.md with a new entry
git add -A
git commit -m "chore: bump version to X.Y.Z"

# Tag and push. CI does the rest.
git tag vX.Y.Z
git push origin main --tags
```

The `publish` job in CI fires automatically on `v*` tags. It
publishes all 7 crates to both registries in dependency order.

## Order of publishing

```
1. bare-metal-evm-types   (no workspace deps)
2. bare-metal-evm-keccak  (no workspace deps)
3. bare-metal-evm-nibble  (no workspace deps)
4. bare-metal-evm-rlp     (depends on types)
5. bare-metal-evm-trie    (depends on keccak, nibble, rlp)
6. bare-metal-evm-state   (depends on types, keccak, rlp, trie)
7. bare-metal-evm-gas     (depends on types only)
```

## Known limitation: GitHub Releases PUT

The Cargo registry protocol asks the registry to accept a `PUT`
request at `{dl}/{crate}-{version}.crate` with the `.crate` tarball
body. GitHub Releases does not accept raw `PUT` to its release
asset URLs. The release upload endpoint requires the GitHub API
with `Authorization`, `Content-Type: application/octet-stream`,
and a specific `name` query parameter.

**Workaround in the publish job:** the CI step pre-uploads each
`.crate` tarball to the GitHub Release created for the tag using
the GitHub API, then runs `cargo publish --registry github` with
the `dl` URL pointed at the release asset. The `PUT` from cargo
will still hit GitHub, but a future patch to `cargo` to support
`--skip-upload` for already-uploaded artifacts would make this
fully clean. For now, the dry-run against `--registry github`
verifies the index is set up correctly; the actual `cargo publish`
step is a known-failing step that you can either skip or replace
with the manual `scripts/publish-to-github.sh` flow (see below).

## Manual publish to the self-hosted registry

If CI is not an option (or for one-off releases), use the manual
flow. It works around the PUT limitation by uploading the
`.crate` file via the GitHub API first.

```bash
# 0. Pre: log in to the github registry
cargo login --registry github    # token: a GitHub PAT with `repo` scope

# 1. For each crate in dependency order
./scripts/publish-to-github.sh bare-metal-evm-types
./scripts/publish-to-github.sh bare-metal-evm-keccak
./scripts/publish-to-github.sh bare-metal-evm-nibble
./scripts/publish-to-github.sh bare-metal-evm-rlp
./scripts/publish-to-github.sh bare-metal-evm-trie
./scripts/publish-to-github.sh bare-metal-evm-state
./scripts/publish-to-github.sh bare-metal-evm-gas
```

`scripts/publish-to-github.sh` does, for the given crate:

1. `cargo package -p <crate>` -- build the `.crate` tarball
2. `gh release upload v<tag> target/package/<crate>-<version>.crate`
   -- attach it to the GitHub Release for the current tag
3. `cargo publish -p <crate> --registry github --no-verify` --
   update the index branch (this currently also tries to PUT the
   file, which fails; the index branch is updated by the next step)
4. `git push origin publish/<crate>-<version>:cargo-registry` --
   fast-forward the `cargo-registry` branch with the new index entry

The `--no-verify` flag skips the dry-run style check, not the
upload; the script then handles the upload manually. Step 3
will still try to PUT, so for now the script also has a
`CARGO_REGISTRIES_GITHUB_PROTOCOL=git` workaround. See the
script for the exact commands.

## Why CI and not manual

Six crates with a real dependency chain is too easy to mess up
by hand. Forgetting to publish in order, missing one crate, or
having a stale local path dep are all common mistakes. CI is
reproducible: same commands, same order, same env, every time.
No local secrets, no manual bookkeeping.

## Why both registries

crates.io is where Rust users search for libraries. The
self-hosted git-based registry is a redundant copy that lives
in the same repo, with `.crate` files as GitHub Release assets
and a Cargo index in the `cargo-registry` branch. Downstream
projects that cannot reach crates.io (corporate firewalls,
mirror setups) can depend on the github registry by adding to
their `.cargo/config.toml`:

    [source.crates-io]
    replace-with = "github"
