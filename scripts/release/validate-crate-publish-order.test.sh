#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

metadata_file="${tmp_dir}/metadata.json"
cat >"${metadata_file}" <<'JSON'
{
  "workspace_members": ["build", "core", "tui", "app", "cli"],
  "packages": [
    {
      "id": "build",
      "name": "codewhale-build-support",
      "version": "0.9.5",
      "dependencies": []
    },
    {
      "id": "core",
      "name": "codewhale-core",
      "version": "0.9.5",
      "dependencies": [
        {"name": "codewhale-cli", "path": "/workspace/cli", "kind": "dev"}
      ]
    },
    {
      "id": "tui",
      "name": "codewhale-tui",
      "version": "0.9.5",
      "dependencies": [
        {"name": "codewhale-build-support", "path": "/workspace/build", "kind": "build"},
        {"name": "codewhale-core", "path": "/workspace/core", "kind": null}
      ]
    },
    {
      "id": "app",
      "name": "codewhale-app-server",
      "version": "0.9.5",
      "dependencies": [
        {"name": "codewhale-core", "path": "/workspace/core", "kind": null}
      ]
    },
    {
      "id": "cli",
      "name": "codewhale-cli",
      "version": "0.9.5",
      "dependencies": [
        {"name": "codewhale-tui", "path": "/workspace/tui", "kind": null},
        {"name": "codewhale-app-server", "path": "/workspace/app", "kind": null}
      ]
    }
  ]
}
JSON

bad_output="${tmp_dir}/bad-order.txt"
if python3 "${script_dir}/validate-crate-publish-order.py" \
  --metadata-file "${metadata_file}" \
  codewhale-build-support \
  codewhale-tui \
  codewhale-core \
  codewhale-app-server \
  codewhale-cli >"${bad_output}" 2>&1; then
  echo "v0.9.5 publication order unexpectedly passed" >&2
  exit 1
fi
grep -F \
  "codewhale-tui (position 2) depends on codewhale-core (position 3) [normal]" \
  "${bad_output}" >/dev/null

good_output="${tmp_dir}/good-order.txt"
python3 "${script_dir}/validate-crate-publish-order.py" \
  --metadata-file "${metadata_file}" \
  codewhale-build-support \
  codewhale-core \
  codewhale-tui \
  codewhale-app-server \
  codewhale-cli >"${good_output}"
grep -F $'version\t0.9.5\t' "${good_output}" >/dev/null
grep -F $'crate\tcodewhale-core\t1' "${good_output}" >/dev/null

# Keep the checked-in order synchronized with the live locked workspace graph.
# shellcheck source=scripts/release/crates.sh
source "${script_dir}/crates.sh"
python3 "${script_dir}/validate-crate-publish-order.py" \
  "${release_crates[@]}" >"${tmp_dir}/workspace-order.txt"

echo "crate publication order validation tests passed"
