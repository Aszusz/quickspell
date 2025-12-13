# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Development
npm run dev          # Start Vite + Tauri dev server
npm run tauri dev    # Run Tauri directly

# Build
npm run build        # Compile TypeScript & Vite build

# Quality
npm run all          # Format + lint everything
npm run lint         # ESLint (TS) + Clippy (Rust, strict)
npm run format       # Prettier (TS) + cargo fmt (Rust)

# Rust only
cargo check          # Fast compile check
cargo test           # Run tests
cargo clippy         # Lint (warnings = errors)
```

## Architecture

This is a Tauri 2 app with React frontend and Rust backend.

### Frontend (`/src`)
- React 19 + TypeScript + Vite + Tailwind CSS
- Path alias: `@/` → `./src/`
- Components use Radix UI primitives in `/components/ui/`

### Backend (`/src-tauri/src`)

**Two-layer structure:**

```
api/           # Tauri interface layer
├── types.rs   # Type definitions only (no impl code)
├── commands.rs# Tauri command handlers (thin wrappers)
└── events.rs  # Event constants + emit function

core/          # Business logic
├── app.rs     # App initialization + spell loading
└── state.rs   # AppState impl (mutations, snapshots, provider execution)

lib.rs         # Tauri setup (tray, hotkeys, window management)
main.rs        # Entry point
```

**Key patterns:**
- `api/types.rs` = pure type definitions, no implementation code
- `api/commands.rs` = thin command handlers that delegate to `core/`
- `core/state.rs` = AppState with Mutex-wrapped inner state
- Frame stack navigation for spell traversal

### IPC

Commands: `start_app`, `get_state_snapshot`
Events: `state-snapshot` (emitted on state changes)

### Resources

`/resources/spells/` - YAML spell definitions loaded at startup
`/resources/providers/` - Provider scripts executed by spells
