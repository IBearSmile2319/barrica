#!/bin/sh
# THIRD_PARTY_NOTICES.md más la lista de bibliotecas de Rust que van en el binario, con su licencia.
set -e
cd "$(dirname "$0")/.."
cat THIRD_PARTY_NOTICES.md
printf '\n### Lista de bibliotecas de Rust\n\n| Biblioteca | Licencia |\n|---|---|\n'
cargo tree -e normal --prefix none --format '{p}|{l}' --target aarch64-apple-darwin 2>/dev/null \
    | grep -v -e '^barrica ' -e '(proc-macro)' | sed 's/ (\*)//' | sort -u \
    | awk -F'|' 'NF == 2 { printf "| %s | %s |\n", $1, $2 }'
