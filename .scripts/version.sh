#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

crates=(oval-cli oval-core oval-format oval-syntax oval-wasm)

cargo_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p;}' Cargo.toml
}

lock_version() {
  awk -v name="$1" '$0 == "name = \"" name "\"" { getline; sub(/^version = "/, ""); sub(/"$/, ""); print; exit }' Cargo.lock
}

package_version() {
  node -p "require('./packages/oval/package.json').version"
}

package_lock_versions() {
  node -p "const lock = require('./packages/oval/package-lock.json'); [lock.version, lock.packages[''].version].join(' ')"
}

check() {
  local expected
  expected="$(cargo_version)"
  local ok=0
  report() {
    if [ "$2" != "$expected" ]; then
      echo "$1 is $2 but Cargo.toml is $expected" >&2
      ok=1
    fi
  }
  for crate in "${crates[@]}"; do
    report "Cargo.lock ($crate)" "$(lock_version "$crate")"
  done
  report "packages/oval/package.json" "$(package_version)"
  for version in $(package_lock_versions); do
    report "packages/oval/package-lock.json" "$version"
  done
  if [ "$ok" -eq 0 ]; then
    echo "every version is $expected"
  fi
  return "$ok"
}

set_version() {
  local version="$1"
  if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "$version is not MAJOR.MINOR.PATCH" >&2
    exit 1
  fi
  sed -i.bak '/^\[workspace\.package\]/,/^\[/s/^version = ".*"$/version = "'"$version"'"/' Cargo.toml
  rm Cargo.toml.bak
  cargo update --workspace --offline --quiet
  (cd packages/oval && npm version "$version" --no-git-tag-version --allow-same-version >/dev/null)
  check
}

case "${1:-}" in
  check) check ;;
  set) set_version "${2:?usage: version.sh set MAJOR.MINOR.PATCH}" ;;
  *) echo "usage: version.sh check | set MAJOR.MINOR.PATCH" >&2; exit 2 ;;
esac
