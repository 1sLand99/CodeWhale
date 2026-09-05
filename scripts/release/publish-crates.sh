#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/release/crates.sh
source "${script_dir}/crates.sh"

mode="${1:-dry-run}"
case "${mode}" in
  dry-run|publish) ;;
  *)
    echo "usage: $0 [dry-run|publish]" >&2
    exit 1
    ;;
esac

packages=("${release_crates[@]}")
crates_user_agent="CodeWhale release publish check (https://github.com/Hmbown/CodeWhale)"

workspace_version=""

metadata_inventory="$(
  python3 "${script_dir}/validate-crate-publish-order.py" "${packages[@]}"
)"
while IFS=$'\t' read -r kind name value; do
  case "${kind}" in
    version)
      workspace_version="${name}"
      ;;
  esac
done <<<"${metadata_inventory}"

if [[ -z "${workspace_version}" ]]; then
  echo "Could not determine workspace version." >&2
  exit 1
fi

echo "Crate publication order OK: ${#packages[@]} workspace crates."

if [[ "${mode}" == "publish" ]]; then
  "${script_dir}/require-release-tag-checkout.sh"
  "${script_dir}/verify-release-assets.sh"
fi

# Package the complete release together. Cargo resolves unpublished workspace
# dependencies through a temporary local registry, then builds each unpacked
# tarball. A file inventory alone cannot detect missing embedded assets.
package_args=(--locked)
if [[ "${mode}" == "dry-run" ]]; then
  package_args+=(--allow-dirty)
fi
for package in "${packages[@]}"; do
  package_args+=(-p "${package}")
done

echo "Verifying all ${#packages[@]} release package tarballs before any upload..."
cargo package "${package_args[@]}"
if [[ "${mode}" == "dry-run" ]]; then
  echo "Release package verification OK; no crates uploaded."
  exit 0
fi

crate_version_exists() {
  local crate_name="$1"
  local crate_version="$2"
  curl -fsSL -A "${crates_user_agent}" "https://crates.io/api/v1/crates/${crate_name}/${crate_version}" >/dev/null 2>&1
}

wait_for_crate_version() {
  local crate_name="$1"
  local crate_version="$2"
  local attempts=30

  for ((attempt = 1; attempt <= attempts; attempt += 1)); do
    if crate_version_exists "${crate_name}" "${crate_version}"; then
      return 0
    fi
    echo "Waiting for ${crate_name} ${crate_version} to appear on crates.io (${attempt}/${attempts})..."
    sleep 10
  done

  echo "Timed out waiting for ${crate_name} ${crate_version} to appear on crates.io" >&2
  return 1
}

for package in "${packages[@]}"; do
  echo "::group::${mode} ${package}"
  if crate_version_exists "${package}" "${workspace_version}"; then
    echo "Skipping ${package} ${workspace_version}; already published."
  else
    cargo publish --locked -p "${package}"
    wait_for_crate_version "${package}" "${workspace_version}"
  fi
  echo "::endgroup::"
done
