#!/usr/bin/env zsh

set -e
set -u
set -o pipefail

# Chrome bookmarks file location
bookmarks_file="$HOME/Library/Application Support/Google/Chrome/Default/Bookmarks"

if [[ ! -f "$bookmarks_file" ]]; then
  echo "Error: Chrome bookmarks file not found at: $bookmarks_file" >&2
  exit 1
fi

# Function to recursively extract bookmarks from JSON
extract_bookmarks() {
  jq -r '
    .. |
    objects |
    select(.type == "url") |
    "\(.name)\t\(.url)"
  ' "$bookmarks_file" | while IFS=$'\t' read -r name url; do
    echo "BOOKMARK\t[B] ${name}\t${url}"
  done
}

extract_bookmarks

exit 0