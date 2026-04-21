#!/usr/bin/env bash
# Regenerate every pinned artifact under artifacts/.
# One script, no hidden state — a clean checkout + `bash tools/build_artifacts.sh`
# must reproduce every committed JSON byte-for-byte (modulo cargo build output).

set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p artifacts/stage01 artifacts/stage02 artifacts/stage03

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

# Stage 2: run traces. Each entry = (name, regex, input).
# Covers match, partial mismatch, trailing-char mismatch, empty input,
# alphabet miss, and a multi-operator regex.
stage02_names=(
    a__match
    a__miss
    ab__match
    ab__partial
    ab__extra
    a_or_b__match_a
    a_or_b__miss
    a_star__empty
    a_star__match
    a_star__miss
    a_or_b_star_c__match
    a_or_b_star_c__miss
)
stage02_regex=(
    'a' 'a'
    'ab' 'ab' 'ab'
    'a|b' 'a|b'
    'a*' 'a*' 'a*'
    '(a|b)*c' '(a|b)*c'
)
stage02_input=(
    'a' 'b'
    'ab' 'a' 'abc'
    'a' 'c'
    '' 'aaa' 'aab'
    'aabc' 'abab'
)

echo ">> cargo build --example 02_run_nfa --release"
cargo build --release --example 02_run_nfa >/dev/null

for i in "${!stage02_names[@]}"; do
    name="${stage02_names[$i]}"
    re="${stage02_regex[$i]}"
    inp="${stage02_input[$i]}"
    out="artifacts/stage02/${name}.json"
    printf '  stage02/%-24s  %q × %q\n' "${name}.json" "$re" "$inp"
    cargo run --quiet --release --example 02_run_nfa -- "$re" "$inp" > "$out"
done

# Stage 3: subset construction. Six pinned regexes covering literal, concat,
# alt (fan-out merging at start), star (start-is-accept), plus (start-is-not-
# accept), and a mixed-operator regex that exercises every alphabet symbol.
stage03_names=(a ab a_or_b a_star a_plus a_or_b_star_c)
stage03_regex=('a' 'ab' 'a|b' 'a*' 'a+' '(a|b)*c')

echo ">> cargo build --example 03_subset_construction --release"
cargo build --release --example 03_subset_construction >/dev/null

for i in "${!stage03_names[@]}"; do
    name="${stage03_names[$i]}"
    re="${stage03_regex[$i]}"
    out="artifacts/stage03/${name}.json"
    printf '  stage03/%-18s  %q\n' "${name}.json" "$re"
    cargo run --quiet --release --example 03_subset_construction -- "$re" > "$out"
done

echo "done."
