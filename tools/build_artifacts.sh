#!/usr/bin/env bash
# Regenerate every pinned artifact under artifacts/.
# One script, no hidden state — a clean checkout + `bash tools/build_artifacts.sh`
# must reproduce every committed JSON byte-for-byte (modulo cargo build output).

set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p artifacts/stage01

# Stage 1: seven pinned regexes covering literal, concat, alt, star, plus,
# alt-under-star, and mixed precedence.
stage01_names=(a ab a_or_b a_star a_plus a_or_b_star a_or_b_star_c)
stage01_regex=('a' 'ab' 'a|b' 'a*' 'a+' '(a|b)*' 'a|b*c')

echo ">> cargo build --example 01_build_nfa --release"
cargo build --release --example 01_build_nfa >/dev/null

for i in "${!stage01_names[@]}"; do
    name="${stage01_names[$i]}"
    re="${stage01_regex[$i]}"
    out="artifacts/stage01/${name}.json"
    printf '  stage01/%-18s  %q\n' "${name}.json" "$re"
    cargo run --quiet --release --example 01_build_nfa -- "$re" > "$out"
done

echo "done."
