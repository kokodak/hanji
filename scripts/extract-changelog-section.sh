#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
	printf "usage: %s VERSION [CHANGELOG]\n" "$0" >&2
	exit 2
fi

version="${1#v}"
changelog="${2:-CHANGELOG.md}"

if [ ! -f "$changelog" ]; then
	printf "Changelog not found: %s\n" "$changelog" >&2
	exit 1
fi

section="$(
	awk -v version="$version" '
		/^## \[/ {
			if (found) {
				exit
			}
			if ($0 ~ "^## \\[" version "\\]") {
				found = 1
			}
		}
		found && /^\[[^]]+\]:/ {
			exit
		}
		found {
			print
		}
	' "$changelog"
)"

if [ -z "$section" ]; then
	printf "No changelog section found for version %s\n" "$version" >&2
	exit 1
fi

printf "%s\n" "$section"
