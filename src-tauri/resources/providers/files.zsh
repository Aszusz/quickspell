#!/usr/bin/env zsh

dir="${1:-$HOME}"

excludes=(
  # macOS system-ish locations (skip whenever encountered)
  -E "**/System/**"
  -E "**/Library/**"
  -E "**/usr/**"
  -E "**/bin/**"
  -E "**/sbin/**"
  -E "**/dev/**"
  -E "**/private/**"
  -E "**/Volumes/**"
  -E "**/Network/**"
  -E "**/cores/**"
  -E "**/Applications/**"
  -E "**/opt/**"

  # Noisy user-level internals
  -E "**/.Trash/**"
  -E "**/Library/Caches/**"
  -E "**/Library/Containers/**"
  -E "**/Library/Group Containers/**"
  -E "**/Library/Logs/**"

  # High-impact dev & app caches (performance critical)
  -E "**/node_modules/**"
  -E "**/.cache/**"
  -E "**/.npm/**"
  -E "**/.cargo/**"
  -E "**/.rustup/**"
  -E "**/.gradle/**"
  -E "**/.m2/**"
  -E "**/.venv/**"
  -E "**/venv/**"
  -E "**/__pycache__/**"
  -E "**/.pytest_cache/**"
  -E "**/.terraform/**"
  -E "**/.next/**"
  -E "**/.yarn/**"
  -E "**/.pnpm-store/**"

  # Build output
  -E "**/build/**"
  -E "**/dist/**"
  -E "**/target/**"
  -E "**/out/**"

  # VCS & misc clutter
  -E "**/.git/**"
  -E "**/.DS_Store"
)

if [[ ! -d "$dir" ]]; then
  echo "Not a directory: $dir" >&2
  exit 1
fi

fd --hidden "${excludes[@]}" --color=never --absolute-path . "$dir" \
| awk '{
  if (substr($0, length($0)) == "/") {
    match($0, /[^\/]+\/$/)
    printf "DIR\t[D] %s\t%s\n", substr($0, RSTART, RLENGTH-1), $0
  } else {
    match($0, /[^\/]+$/)
    printf "FILE\t[F] %s\t%s\n", substr($0, RSTART, RLENGTH), $0
  }
}'
