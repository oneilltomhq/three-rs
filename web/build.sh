#!/usr/bin/env bash
#
# Builds the browser shell into web/dist/ (git-ignored).
#
#   web/build.sh            # release
#   web/build.sh --debug    # a much larger .wasm with readable panic traces
#
# Two steps and no bundler: cargo makes the .wasm, wasm-bindgen writes the JS
# glue that calls into it, and index.html is copied beside them. The
# wasm-bindgen CLI's version must match the `wasm-bindgen` crate in Cargo.lock
# exactly — they are one program split in two — so this checks rather than
# letting a mismatch fail at run time with an unreadable error.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
dist="$here/dist"

profile="release"
target_dir="release"
if [[ "${1:-}" == "--debug" ]]; then
    profile="debug"
    target_dir="debug"
fi

want="$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;q}' "$root/Cargo.lock")"
if ! command -v wasm-bindgen >/dev/null 2>&1; then
    echo "wasm-bindgen-cli is not on PATH. Install the matching version:" >&2
    echo "    cargo install wasm-bindgen-cli --version $want --locked" >&2
    exit 1
fi
have="$(wasm-bindgen --version | awk '{print $2}')"
if [[ "$have" != "$want" ]]; then
    echo "wasm-bindgen-cli is $have but Cargo.lock pins wasm-bindgen $want." >&2
    echo "    cargo install wasm-bindgen-cli --version $want --locked" >&2
    exit 1
fi

cd "$root"
if [[ "$profile" == "release" ]]; then
    cargo build --release --target wasm32-unknown-unknown -p three-rs-web
else
    cargo build --target wasm32-unknown-unknown -p three-rs-web
fi

rm -rf "$dist"
mkdir -p "$dist"

# `--target web` emits an ES module with a default-export initialiser, which is
# what index.html's `import("./three_rs_web.js")` expects. No --no-modules and
# no bundler step.
wasm-bindgen \
    --target web \
    --out-dir "$dist" \
    --out-name three_rs_web \
    "$root/target/wasm32-unknown-unknown/$target_dir/three_rs_web.wasm"

cp "$here/index.html" "$dist/index.html"

echo
echo "built $dist ($profile):"
ls -la "$dist"
echo
echo "serve it with:  python3 -m http.server --directory $dist 8000"
echo "then open:      http://localhost:8000/"
