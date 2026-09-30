#!/usr/bin/env bash
# The workspace's integration tests, split by `tests/gpu_only` (#227).
#
#   tools/ci_tests.sh no-gpu   every test target not in tests/gpu_only
#   tools/ci_tests.sh gpu      the tests/gpu_only targets with no reason given,
#                              one test thread at a time
#
# CI's `test` job runs the first and its `lavapipe` job the second; both run
# locally too. Every package runs with --no-fail-fast, so one failing target
# does not hide the rest.
set -euo pipefail
cd "$(dirname "$0")/.."

mode=${1:?usage: tools/ci_tests.sh no-gpu|gpu}
case $mode in no-gpu | gpu) ;; *) echo "unknown mode $mode" >&2; exit 2 ;; esac

targets=$(cargo metadata --no-deps --format-version 1 |
	jq -r '.packages[] | .name as $p | .targets[] | select(.kind == ["test"]) | "\($p)/\(.name)"')

declare -A listed has_reason
while read -r name reason; do
	[[ -z $name || $name == \#* ]] && continue
	if ! grep -qxF "$name" <<<"$targets"; then
		echo "tests/gpu_only lists $name, which is not a test target" >&2
		exit 1
	fi
	listed[$name]=1
	if [[ -n $reason ]]; then has_reason[$name]=1; fi
done <tests/gpu_only

# Whether this mode runs test target $1.
runs() {
	if [[ $mode == no-gpu ]]; then
		[[ ! -v listed["$1"] ]]
	else
		[[ -v listed["$1"] && ! -v has_reason["$1"] ]]
	fi
}

failed=0
for pkg in $(cut -d/ -f1 <<<"$targets" | sort -u); do
	args=()
	for t in $(grep "^$pkg/" <<<"$targets"); do
		if runs "$t"; then args+=(--test "${t#*/}"); fi
	done
	((${#args[@]})) || continue
	if [[ $mode == gpu ]]; then
		cargo test -p "$pkg" --no-fail-fast "${args[@]}" -- --test-threads=1 || failed=1
	else
		cargo test -p "$pkg" --no-fail-fast "${args[@]}" || failed=1
	fi
done
exit $failed
