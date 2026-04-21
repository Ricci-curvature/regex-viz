#!/usr/bin/env bash
# Regenerate every pinned artifact under artifacts/.
# One script, no hidden state — a clean checkout + `bash tools/build_artifacts.sh`
# must reproduce every committed JSON byte-for-byte (modulo cargo build output).

set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p artifacts/stage01 artifacts/stage02 artifacts/stage03 artifacts/stage04 artifacts/stage05

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

# Stage 4: NFA vs DFA side-by-side on the same (regex, input) pairs used in
# Stage 2 — reusing the pin set keeps the comparison honest (every verdict
# must match the standalone matcher).
stage04_names=(
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
stage04_regex=(
    'a' 'a'
    'ab' 'ab' 'ab'
    'a|b' 'a|b'
    'a*' 'a*' 'a*'
    '(a|b)*c' '(a|b)*c'
)
stage04_input=(
    'a' 'b'
    'ab' 'a' 'abc'
    'a' 'c'
    '' 'aaa' 'aab'
    'aabc' 'abab'
)

echo ">> cargo build --example 04_compare_nfa_dfa --release"
cargo build --release --example 04_compare_nfa_dfa >/dev/null

for i in "${!stage04_names[@]}"; do
    name="${stage04_names[$i]}"
    re="${stage04_regex[$i]}"
    inp="${stage04_input[$i]}"
    out="artifacts/stage04/${name}.json"
    printf '  stage04/%-24s  %q × %q\n' "${name}.json" "$re" "$inp"
    cargo run --quiet --release --example 04_compare_nfa_dfa -- "$re" "$inp" > "$out"
done

# Stage 5: Hopcroft DFA minimization. Six carryover pins from Stage 3 (for
# continuity — same regexes, now with the minimal DFA beside them) plus three
# teaching pins that specifically show block merges:
#   - aa|ab          → two symmetric leaf accepts merge
#   - abc|axc        → symmetric inner states merge
#   - (a|b)(a|b)     → strings of length exactly 2 over {a,b}; merges by length
stage05_names=(
    a
    ab
    a_or_b
    a_star
    a_plus
    a_or_b_star_c
    aa_or_ab
    abc_or_axc
    a_or_b_twice
)
stage05_regex=(
    'a'
    'ab'
    'a|b'
    'a*'
    'a+'
    '(a|b)*c'
    'aa|ab'
    'abc|axc'
    '(a|b)(a|b)'
)

echo ">> cargo build --example 05_minimize_dfa --release"
cargo build --release --example 05_minimize_dfa >/dev/null

for i in "${!stage05_names[@]}"; do
    name="${stage05_names[$i]}"
    re="${stage05_regex[$i]}"
    out="artifacts/stage05/${name}.json"
    printf '  stage05/%-24s  %q\n' "${name}.json" "$re"
    cargo run --quiet --release --example 05_minimize_dfa -- "$re" > "$out"
done

echo "done."
