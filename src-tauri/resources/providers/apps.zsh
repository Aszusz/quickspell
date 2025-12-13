#!/usr/bin/env zsh

app_dirs=(/Applications /System/Applications)

fd -t d -d 1 '\.app$' "${app_dirs[@]}" -0 \
| xargs -0 mdls -name kMDItemPath -name kMDItemLastUsedDate -- \
| sd '(?ms)kMDItemLastUsedDate = (.*?)\nkMDItemPath\s+= "(.*?)"' '$2\t$1' \
| sd '^\s+' '' \
| sort -t$'\t' -k2 -r \
| cut -f1 \
| sd '^.*/([^/]+)\.app$' 'APP\t[A] $1\t$0'