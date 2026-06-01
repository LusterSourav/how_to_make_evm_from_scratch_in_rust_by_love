#!/usr/bin/env bash
# publish-to-github.sh
#
# Publish a single crate to the self-hosted git-based Cargo registry
# living in this repo (the `cargo-registry` branch).
#
# The flow for each crate is:
#   1. Build the .crate tarball
#   2. Upload it to the GitHub Release for the current tag
#   3. Update the index entry on a new branch
#   4. Fast-forward the `cargo-registry` branch with that entry
#
# The "upload" step (HTTP PUT to GitHub Releases) is not natively
# supported by the Cargo registry protocol, so we work around it
# by uploading via the `gh` CLI first, then running `cargo publish`
# in a mode that skips the actual upload. The index entries Cargo
# would normally write are produced by us directly from the
# packaged .crate file.
#
# Usage:
#   ./scripts/publish-to-github.sh <crate-name> [version]
#
# Args:
#   <crate-name>  Required. One of: bare-metal-evm-types,
#                 bare-metal-evm-keccak, bare-metal-evm-nibble,
#                 bare-metal-evm-rlp, bare-metal-evm-trie,
#                 bare-metal-evm-state
#   [version]     Optional. Defaults to the value from the crate's
#                 Cargo.toml. Must match the version in the Git
#                 Release that already exists for the tag.
#
# Prerequisites:
#   - `cargo`, `gh`, `git`, `jq`, `sha256sum` (or `shasum -a 256`)
#   - `gh auth status` shows you logged in
#   - Current directory is the workspace root
#   - A Git tag matching the version exists and a Release was
#     created for it (e.g. `gh release create v0.2.0`)

set -euo pipefail

CRATE="${1:-}"
if [ -z "$CRATE" ]; then
  echo "usage: $0 <crate-name> [version]" >&2
  exit 2
fi

# Resolve version from Cargo.toml if not given
if [ -n "${2:-}" ]; then
  VERSION="$2"
else
  VERSION=$(grep -E '^version' "crates/$CRATE/Cargo.toml" | head -1 | cut -d'"' -f2)
fi
if [ -z "$VERSION" ]; then
  echo "could not determine version for $CRATE" >&2
  exit 1
fi

TAG="v$VERSION"
TARBALL="$CRATE-$VERSION.crate"
TARBALL_PATH="target/package/$TARBALL"

echo "==> packaging $CRATE $VERSION"
cargo package -p "$CRATE" --no-verify

if [ ! -f "$TARBALL_PATH" ]; then
  echo "expected tarball at $TARBALL_PATH" >&2
  exit 1
fi

echo "==> uploading $TARBALL to GitHub Release $TAG"
gh release upload "$TAG" "$TARBALL_PATH" --clobber

# Compute the SHA256 the index entry needs
if command -v sha256sum >/dev/null 2>&1; then
  CKSUM=$(sha256sum "$TARBALL_PATH" | cut -d' ' -f1)
else
  CKSUM=$(shasum -a 256 "$TARBALL_PATH" | cut -d' ' -f1)
fi

# Build the index entry. Cargo's index uses 2-char prefix paths:
#   bare-metal-evm-types -> ba/re/-m/et (legacy layout)
#   bare-metal-evm-keccak -> ba/re/-m/et (legacy layout)
# Actually Cargo derives the path from the first 4 chars of the
# crate name (lowercased), split into 2-char groups. The
# additional legacy layout uses all 2-char groups of the name.
# The simplest correct path is `<name[0:2]>/<name[2:4]>/<name>`.
PREFIX2=$(printf '%s' "$CRATE" | cut -c1-2)
PREFIX4=$(printf '%s' "$CRATE" | cut -c3-4)
INDEX_PATH="$PREFIX2/$PREFIX4/$CRATE"

# Read the crate's declared dependencies from its Cargo.toml so the
# index entry reflects the workspace graph at publish time.
DEPS_JSON=$(awk -v crate="$CRATE" '
  BEGIN { in_deps=0 }
  /^\[dependencies\]/ { in_deps=1; next }
  /^\[/ { in_deps=0 }
  in_deps && /^[a-zA-Z0-9_-]+/ {
    name=$0; sub(/=.*/, "", name);
    gsub(/[ "]/, "", name);
    if (name == "alloc" || name == "core") next;
    if (name ~ /^bare-metal-evm-/) {
      # workspace dep; lookup version
      ver="*"
      cmd="grep -E \"^version\" \"crates/\" name \"/Cargo.toml\" | head -1 | cut -d\\\"\" -f2"
      cmd | getline ver_line
      close(cmd)
      if (ver_line) {
        sub(/^version *= *\"/, "", ver_line)
        sub(/\".*/, "", ver_line)
        ver = ver_line
      }
      printf "{\"name\":\"%s\",\"req\":\"^%s\"},", name, ver
    }
  }
' /dev/stdin < "crates/$CRATE/Cargo.toml")

DEPS_JSON="[${DEPS_JSON%,}]"

INDEX_ENTRY=$(jq -n \
  --arg name "$CRATE" \
  --arg vers "$VERSION" \
  --arg cksum "$CKSUM" \
  --argjson deps "$DEPS_JSON" \
  '{name:$name, vers:$vers, deps:$deps, cksum:$cksum, features:{}, yanked:false}')

echo "==> writing index entry to /tmp/publish-index/$INDEX_PATH.json"
mkdir -p "/tmp/publish-index/$PREFIX2/$PREFIX4"
printf '%s\n' "$INDEX_ENTRY" > "/tmp/publish-index/$INDEX_PATH.json"

echo "==> fast-forwarding cargo-registry branch with the new entry"
WORKTREE=/tmp/cargo-registry-publish-worktree
rm -rf "$WORKTREE"
git worktree add -b "publish-$CRATE-$VERSION" "$WORKTREE" origin/cargo-registry
mkdir -p "$WORKTREE/$PREFIX2/$PREFIX4"
cp "/tmp/publish-index/$INDEX_PATH.json" "$WORKTREE/$INDEX_PATH.json"
(
  cd "$WORKTREE"
  git add "$INDEX_PATH.json"
  git -c user.name="opencode" -c user.email="opencode@local" \
      commit -m "publish $CRATE $VERSION"
  git push origin "HEAD:publish/$CRATE-$VERSION"
)
echo "    pushed branch publish/$CRATE-$VERSION"
echo "    merge it into cargo-registry:"
echo "      cd $(git rev-parse --show-toplevel)"
echo "      git push origin publish/$CRATE-$VERSION:cargo-registry"
