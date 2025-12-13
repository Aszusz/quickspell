#!/usr/bin/env zsh
# providers/spells.zsh
# Usage: ./spells.zsh [--all|--enabled|--disabled]

set -e
set -u
set -o pipefail

data_dir="${GOSPELL_DATA_DIR:-.}"
spells_dir="${SPELLS_DIR:-${data_dir%/}/spells}"
spells_dir="${spells_dir:-./spells}"
filter_mode="${1:---all}"

if [[ ! -d "$spells_dir" ]]; then
  echo "Error: Spells directory not found: $spells_dir" >&2
  exit 1
fi

# Validate filter mode
case "$filter_mode" in
  --all|--enabled|--disabled) ;;
  *)
    echo "Usage: $0 [--all|--enabled|--disabled]" >&2
    exit 1
    ;;
esac

# Find all spell YAML files and process them in parallel
fd -e yml -e yaml . "$spells_dir" -0 \
| xargs -0 -n1 -P4 sh -c '
  file="$1"
  
  # Extract fields using yq (one field at a time to avoid multiline issues)
  name=$(yq -r ".name" "$file")
  id=$(yq -r ".id" "$file")
  alias=$(yq -r ".alias" "$file")
  enabled=$(yq -r ".enabled" "$file")
  
  # Build display string with alias if present
  if [ "$alias" != "null" ] && [ -n "$alias" ]; then
    display="[S] $name ($alias)"
  else
    display="[S] $name"
  fi
  
  # Output with enabled status for filtering
  echo "$enabled\tSPELL\t$display\t$id"
' _ \
| {
  # Filter based on mode
  case "$filter_mode" in
    --enabled)
      grep "^true"
      ;;
    --disabled)
      grep "^false"
      ;;
    --all)
      cat
      ;;
  esac
} \
| cut -f2- \
| sort -t$'\t' -k2
