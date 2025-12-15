# Critical Bugs

- Pagination desync hides items beyond the first 100 (`src/App.tsx:112-118`): the backend exposes full `filtered_items`, but the frontend derives paging from `topItems`, which is limited to 100 entries. When `selectedIndex` moves past the first 100 items (ArrowDown), `pageItems` becomes empty and the UI shows “No items loaded” even though selection continues on hidden items. Items after the first 100 cannot be viewed or acted on until paging uses the full list.
