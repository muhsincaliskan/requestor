#!/bin/sh
set -eu

tag="${1:?usage: scripts/release-checksums.sh <tag>}"
checksums="SHA256SUMS"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

gh release download "$tag" --dir "$work" --pattern '*' --skip-existing
rm -f "$work/$checksums"
(
  cd "$work"
  for asset in *; do
    shasum -a 256 "$asset"
  done
) > "$work.sums"
mv "$work.sums" "$work/$checksums"
gh release upload "$tag" "$work/$checksums" --clobber
cat "$work/$checksums"
