#!/usr/bin/env bash
# Fetch the optional real-world `.atn` corpus used by
# `pictura-codec::real_atn_corpus_round_trips_when_present`.
#
# The corpus is test input only: it is downloaded into a cache directory and is
# never committed. The test self-skips when `PICTURA_ATN_CORPUS` is unset.
#
# Source: LongMarch7/json-photoshop-scripting (GPL-3.0), commit
# 2af118471517433c6c3c1ffa41119ce2d4a1fcda,
# `JSON-Action-Manager/tests/resources/Cross Processing.atn`.
#
# Usage:
#   scripts/fetch-atn-corpus.sh [destination]
#   export PICTURA_ATN_CORPUS="$(scripts/fetch-atn-corpus.sh)"
set -euo pipefail

default_dest="${XDG_CACHE_HOME:-$HOME/.cache}/pictura/atn-corpus"
dest="${1:-$default_dest}"
mkdir -p "$dest"

fetch() {
    local url="$1" name="$2" sha256="$3"
    local target="$dest/$name"
    local tmp
    tmp="$(mktemp "$dest/.atn-fetch.XXXXXX")"
    printf 'fetching %s\n' "$name" >&2
    if ! curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
        --retry 3 --output "$tmp" "$url"; then
        rm -f "$tmp"
        exit 1
    fi
    if ! printf '%s  %s\n' "$sha256" "$tmp" | sha256sum --check --status -; then
        printf 'checksum mismatch for %s\n' "$name" >&2
        rm -f "$tmp"
        exit 1
    fi
    if ! mv -f "$tmp" "$target"; then
        rm -f "$tmp"
        exit 1
    fi
}

fetch \
    'https://raw.githubusercontent.com/LongMarch7/json-photoshop-scripting/2af118471517433c6c3c1ffa41119ce2d4a1fcda/JSON-Action-Manager/tests/resources/Cross%20Processing.atn' \
    'Cross Processing.atn' \
    '26f64007e489b221e24a67db9205faaddd668878d305f5dd57d98dcfa08fc1de'

printf '%s\n' "$dest"
