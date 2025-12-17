# QuickSpell

A macOS-only, keyboard-first command palette built with Tauri. One fast main search for common actions (apps, commands, links) plus specialized “spells” for heavier searches (files, bookmarks) and multi-step workflows.

## What it is
- Global palette (`Ctrl+Space`) that hides when unfocused; centered, minimal UI with on-screen hints.
- Mixed main search: fuzzy matches apps, commands, and links without choosing categories first.
- Specialized spells for big data sets (files, bookmarks) and chained flows (“Search Files” → “Open With”).
- Optional actions (Ctrl+O) for the current item; main action on Enter.
- Extensible via YAML spells backed by tiny Zsh provider scripts.

## How it works
- Providers output tab-delimited rows: `TYPE<TAB>DISPLAY<TAB>PAYLOAD` (see `src-tauri/resources/providers/`).
- Spells (YAML in `src-tauri/resources/spells/`) wire providers to actions:
  - `CMD` runs a shell command.
  - `SPELL` jumps to another spell, enabling layered workflows.
- At first launch, spells/providers are copied to `~/Library/Application Support/com.github.aszusz.quickspell/{spells,providers}` so you can customize safely.

## Philosophy (short)
- Search is the interface; minimize keystrokes.
- Fuzzy by default, quote for exact (`"chrome"`).
- Discoverable patterns over chord memorization; contextual key hints only.

## Requirements (macOS)
- Node.js 20+, npm.
- Rust (stable) with Xcode Command Line Tools.
- CLI tools used by providers: `zsh`, `fd`, `sd`, `jq`, `yq`, plus macOS `mdls`.

## Develop
```bash
npm ci
npm run tauri dev   # Vite + Tauri dev
```

## Build
```bash
npm run tauri build
```
Artifacts land in `src-tauri/target/release/bundle/`.

### If macOS blocks the download (quarantine)
The DMG is unsigned. After download, clear quarantine before opening:
```bash
xattr -dr com.apple.quarantine /path/to/QuickSpell*.dmg
open /path/to/QuickSpell*.dmg
```
Without that, Finder may refuse to open the installer.

## Keyboard
- `Ctrl+Space` toggle palette (global)
- `Enter` main action
- `Ctrl+O` optional actions
- `↑ / ↓` select, `Esc` go back/close

## CI/CD
- `.github/workflows/build.yml`: macOS lint + build, uploads bundle artifact.
- `.github/workflows/release.yml`: manual release (version input), reuses build, drafts a tagged release with artifacts attached.
