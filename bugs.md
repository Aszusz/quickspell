# Critical Bugs

- Shell command injection in action execution (`src-tauri/src/core/state.rs:461-482`): rendered spell/action templates are passed directly into `sh -c` without any escaping. Item data comes from the filesystem/providers, so selecting a file/spell with characters like quotes, backticks, or `$(…)` will be executed when the user runs the action (e.g., a filename `"; touch /tmp/pwn "` triggers that command). Needs shell-free execution or robust escaping of user-controlled fields.

- Pagination desync hides items beyond the first 100 (`src/App.tsx:112-118`): the backend exposes full `filtered_items`, but the frontend derives paging from `topItems`, which is limited to 100 entries. When `selectedIndex` moves past the first 100 items (ArrowDown), `pageItems` becomes empty and the UI shows “No items loaded” even though selection continues on hidden items. Items after the first 100 cannot be viewed or acted on until paging uses the full list.
