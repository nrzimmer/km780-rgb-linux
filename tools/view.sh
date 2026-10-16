#!/usr/bin/env bash
# view.sh <extracted-file> <frame>...   prints the payload in 16-byte rows
F="$1"; shift
for n in "$@"; do
  awk -F'\t' -v n="$n" '$1==n {
    printf "--- frame %s  t=%s  %s  cmd=%s ---\n", $1, $2, $3, $4
    for (i = 1; i <= length($5); i += 32) {
      printf "%04d  ", (i-1)/2
      row = substr($5, i, 32)
      for (j = 1; j <= length(row); j += 2) printf "%s ", substr(row, j, 2)
      printf "\n"
    }
  }' "$F"
done
