#!/usr/bin/env bash
# Opportunity audit — fetch and build the six frozen applications.
#
# Versions are pinned here and their tarball hashes appended to
# $AUDIT/versions.txt. The build is the application's own, with clang -O3 and
# default flags (auto-vectorization ON) — see AUDIT_PREREGISTRATION.md addendum 1
# for why this is the baseline and why the unvectorized configuration is only a
# labelled diagnostic.
#
#   scripts/audit-fetch.sh [DEST]     # default /tmp/audit
set -uo pipefail

A="${1:-/tmp/audit}"
SRC="$A/src"; BLD="$A/build"; DL="$A/dl"
mkdir -p "$SRC" "$BLD" "$DL"
: > "$A/versions.txt"

CC=clang
export CC
export CFLAGS="-O3"

log() { printf '\n=== %s ===\n' "$*"; }

get() { # url dest
    [ -f "$2" ] || curl -fsSL --max-time 600 "$1" -o "$2" || { echo "FETCH FAIL $1"; return 1; }
    printf '%s  %s\n' "$(sha256sum "$2" | cut -d' ' -f1)" "$(basename "$2")" >> "$A/versions.txt"
}

untar() { # archive destname
    [ -d "$SRC/$2" ] || { mkdir -p "$SRC/$2"; tar -xf "$1" -C "$SRC/$2" --strip-components=1; }
}

# ---------------------------------------------------------------- A1 GNU grep
log "A1 grep 3.12"
grep_ver=3.12
get "https://ftp.gnu.org/gnu/grep/grep-$grep_ver.tar.xz" "$DL/grep-$grep_ver.tar.xz" \
    && untar "$DL/grep-$grep_ver.tar.xz" grep \
    && (cd "$SRC/grep" && ./configure --disable-nls >/dev/null 2>&1 && make -j2 >/dev/null 2>&1) \
    && cp "$SRC/grep/src/grep" "$BLD/A1-grep" && echo "BUILT A1-grep" \
    || echo "FAIL A1"

# ------------------------------------------------------------------- A2 zstd
log "A2 zstd v1.5.7"
zstd_ver=1.5.7
get "https://github.com/facebook/zstd/releases/download/v$zstd_ver/zstd-$zstd_ver.tar.gz" \
    "$DL/zstd-$zstd_ver.tar.gz" \
    && untar "$DL/zstd-$zstd_ver.tar.gz" zstd \
    && (cd "$SRC/zstd" && make -j2 >/dev/null 2>&1) \
    && cp "$SRC/zstd/programs/zstd" "$BLD/A2-zstd" && echo "BUILT A2-zstd" \
    || echo "FAIL A2"

# --------------------------------------------------------------------- A3 jq
log "A3 jq 1.8.2"
jq_ver=1.8.2
get "https://github.com/jqlang/jq/releases/download/jq-$jq_ver/jq-$jq_ver.tar.gz" \
    "$DL/jq-$jq_ver.tar.gz" \
    && untar "$DL/jq-$jq_ver.tar.gz" jq \
    && (cd "$SRC/jq" && ./configure --disable-maintainer-mode --disable-shared >/dev/null 2>&1 \
        && make -j2 >/dev/null 2>&1) \
    && cp "$SRC/jq/jq" "$BLD/A3-jq" \
    && head -c 4 "$BLD/A3-jq" | od -An -c | grep -q '177   E   L   F' \
    && echo "BUILT A3-jq" || echo "FAIL A3 (not an ELF binary)"
# --disable-shared above matters: with a shared libjq, the `jq` in the build
# root is a libtool *wrapper script*, and copying it silently yields a 6 KB
# bash script that profiles as bash rather than as jq.

# ----------------------------------------------------------------- A4 sqlite
log "A4 sqlite 3.53.4 amalgamation"
sq_ver=3530400
get "https://www.sqlite.org/2026/sqlite-amalgamation-$sq_ver.zip" "$DL/sqlite-$sq_ver.zip" \
    && { [ -d "$SRC/sqlite" ] || { mkdir -p "$SRC/sqlite" && unzip -q -o "$DL/sqlite-$sq_ver.zip" -d "$SRC/sqlite"; }; } \
    && clang -O3 -o "$BLD/A4-sqlite3" \
         "$SRC/sqlite"/*/shell.c "$SRC/sqlite"/*/sqlite3.c -lpthread -ldl -lm 2>/dev/null \
    && echo "BUILT A4-sqlite3" || echo "FAIL A4"

# ------------------------------------------------------------------ A5 libpng
log "A5 libpng v1.6.58"
png_ver=1.6.58
get "https://github.com/pnggroup/libpng/archive/refs/tags/v$png_ver.tar.gz" "$DL/libpng-$png_ver.tar.gz" \
    && untar "$DL/libpng-$png_ver.tar.gz" libpng \
    && (cd "$SRC/libpng" && cmake -S . -B build -DCMAKE_C_COMPILER=clang \
            -DCMAKE_C_FLAGS="-O3" -DPNG_SHARED=OFF -DPNG_TESTS=ON >/dev/null 2>&1 \
        && cmake --build build -j2 >/dev/null 2>&1) \
    && clang -O3 -I "$SRC/libpng" -I "$SRC/libpng/build" \
         "$SRC/libpng/pngtest.c" "$SRC/libpng/build/libpng.a" -lz -lm \
         -o "$BLD/A5-pngtest" >/dev/null 2>&1 \
    && echo "BUILT A5-pngtest" || echo "FAIL A5"

# --------------------------------------------------------------------- A6 lua
log "A6 lua 5.5.1"
lua_ver=5.5.1
get "https://www.lua.org/ftp/lua-$lua_ver.tar.gz" "$DL/lua-$lua_ver.tar.gz" \
    && untar "$DL/lua-$lua_ver.tar.gz" lua \
    && (cd "$SRC/lua" && make -j2 posix >/dev/null 2>&1) \
    && cp "$SRC/lua/src/lua" "$BLD/A6-lua" && echo "BUILT A6-lua" \
    || echo "FAIL A6"

log "binaries"
ls -la "$BLD"
