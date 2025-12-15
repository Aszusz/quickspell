import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Item as SpellItem, StateSnapshot } from "./events";
import { listenEvent } from "./events";
import { useOsTheme } from "./hooks/use-os-theme";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "./components/ui/breadcrumb";
import { Input } from "./components/ui/input";
import { Search } from "lucide-react";
import { Item, ItemGroup, ItemTitle } from "./components/ui/item";
import { Kbd, KbdGroup } from "./components/ui/kbd";
import { usePaginationLayout } from "./hooks/use-pagination-layout";
import { cn } from "./lib/utils";
import { Spinner } from "./components/ui/spinner";

const DEFAULT_SNAPSHOT: StateSnapshot = {
  status: "loading",
  noOfSpells: 0,
  totalItems: 0,
  spellNames: [],
  topItems: [],
  query: "",
  isFiltering: false,
  selectedItem: null,
};

function App() {
  const [snapshot, setSnapshot] = useState<StateSnapshot>(DEFAULT_SNAPSHOT);
  const [isActionsOpen, setIsActionsOpen] = useState(false);
  const [dialogItem, setDialogItem] = useState<SpellItem | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useOsTheme();
  const { containerRef, measureItemRef, pageSize } = usePaginationLayout({
    estimatedItemHeight: 44,
    gap: 8,
  });

  const selectedItem = snapshot.selectedItem?.details ?? null;
  const selectedActions = snapshot.selectedItem?.actions ?? [];
  const selectedIndex = snapshot.selectedItem?.index ?? 0;

  const closeActionsDialog = useCallback(() => {
    setIsActionsOpen(false);
    setDialogItem(null);
    requestAnimationFrame(() => searchRef.current?.focus());
  }, []);

  const openActionsDialog = useCallback(
    (item?: SpellItem | null) => {
      const target = item ?? selectedItem;
      if (!target) return;
      setDialogItem(target);
      setIsActionsOpen(true);
    },
    [selectedItem]
  );

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    const bootstrap = async () => {
      try {
        const unlisten = await listenEvent("state-snapshot", (payload) => {
          setSnapshot(payload);
        });
        cleanup = unlisten;

        await invoke("start_app");

        const latest = await invoke<StateSnapshot>("get_state_snapshot");
        setSnapshot(latest);
      } catch (err) {
        console.error(err);
      }
    };

    bootstrap();

    return () => {
      cleanup?.();
    };
  }, []);

  const spellNames = useMemo(
    () => [...snapshot.spellNames].sort((a, b) => a.localeCompare(b)),
    [snapshot.spellNames]
  );

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isActionsOpen) {
        if (e.key === "Escape") {
          e.preventDefault();
          closeActionsDialog();
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === "o" || e.key === "O")) {
        e.preventDefault();
        openActionsDialog();
        return;
      }

      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const delta = e.key === "ArrowDown" ? 1 : -1;
        invoke("set_selection_delta", { delta });
        return;
      }

      if (e.key === "Escape") {
        e.preventDefault();
        invoke("handle_escape").catch((err) => {
          console.error("failed to handle escape", err);
        });
        return;
      }

      if (e.key === "Enter") {
        e.preventDefault();
        void invoke("invoke_action", { label: "MAIN" }).catch((err) => {
          console.error("failed to invoke MAIN action", err);
        });
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [closeActionsDialog, isActionsOpen, openActionsDialog]);

  const handleSearchBlur = () => {
    // Keep focus on the search box even after clicking outside.
    requestAnimationFrame(() => searchRef.current?.focus());
  };

  const items = snapshot.topItems;
  const totalItems = items.length;
  const effectivePageSize = Math.max(1, pageSize);
  const currentPage = totalItems ? Math.floor(selectedIndex / effectivePageSize) : 0;
  const pageCount = totalItems ? Math.ceil(totalItems / effectivePageSize) : 0;
  const pageStart = currentPage * effectivePageSize;
  const pageItems = totalItems ? items.slice(pageStart, pageStart + effectivePageSize) : [];
  const showSpinner =
    snapshot.status === "booting" || snapshot.status === "loading" || snapshot.isFiltering;

  return (
    <main className="bg-background text-foreground flex h-screen w-full flex-col overflow-hidden p-3 sm:p-4">
      <div className="flex min-h-0 flex-1 flex-col gap-6">
        <div className="flex items-start justify-between gap-3">
          <div className="w-full space-y-2">
            <Breadcrumb className="border-border/80 bg-muted/40 w-full rounded-lg border px-3 py-2">
              <BreadcrumbList>
                {spellNames.length === 0 && (
                  <BreadcrumbItem>
                    <BreadcrumbPage>Spells</BreadcrumbPage>
                  </BreadcrumbItem>
                )}
                {spellNames.map((name, idx) => (
                  <React.Fragment key={name}>
                    <BreadcrumbItem>
                      {idx === spellNames.length - 1 ? (
                        <BreadcrumbPage>{name}</BreadcrumbPage>
                      ) : (
                        <span className="text-foreground/80 text-sm">{name}</span>
                      )}
                    </BreadcrumbItem>
                    {idx < spellNames.length - 1 ? <BreadcrumbSeparator /> : null}
                  </React.Fragment>
                ))}
              </BreadcrumbList>
            </Breadcrumb>
          </div>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-2">
          <div className="relative w-full">
            {showSpinner ? (
              <Spinner className="text-muted-foreground absolute top-1/2 left-3 -translate-y-1/2" />
            ) : (
              <Search className="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2" />
            )}
            <Input
              ref={searchRef}
              className="w-full pr-14 pl-10"
              placeholder="Type to search..."
              onBlur={handleSearchBlur}
              value={snapshot.query}
              onChange={(e) => {
                const value = e.target.value;
                // Optimistically update local snapshot to keep typing responsive.
                setSnapshot((prev) => ({ ...prev, query: value }));
                invoke("set_query", { query: value });
              }}
            />
            <span className="text-muted-foreground pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs font-medium select-none">
              {snapshot.totalItems}
            </span>
          </div>

          <section className="flex min-h-0 flex-1 flex-col">
            {pageItems.length ? (
              <div
                ref={containerRef}
                className="bg-muted/40 min-h-0 w-full flex-1 overflow-hidden rounded-none"
              >
                <ItemGroup className="gap-2">
                  {pageItems.map((item, idx) => {
                    const absoluteIdx = pageStart + idx;
                    return (
                      <Item
                        key={`${item.Type}-${item.Data}-${absoluteIdx}`}
                        ref={idx === 0 ? measureItemRef : undefined}
                        size="sm"
                        variant="muted"
                        className="data-[selected=true]:bg-primary/10 data-[selected=true]:border-primary/50 border-border/80 border px-3 py-2"
                        data-selected={selectedIndex === absoluteIdx}
                        aria-selected={selectedIndex === absoluteIdx}
                      >
                        <ItemTitle className="font-mono text-xs">{item.Name}</ItemTitle>
                      </Item>
                    );
                  })}
                </ItemGroup>
              </div>
            ) : (
              <div className="text-muted-foreground text-sm">No items loaded</div>
            )}
            {pageCount > 1 ? (
              <div className="flex items-center gap-2 pt-2">
                <nav aria-label="Pagination" className="flex items-center gap-2">
                  {Array.from({ length: pageCount }).map((_, idx) => (
                    <span
                      key={idx}
                      aria-current={idx === currentPage ? "page" : undefined}
                      className={cn(
                        "border-border/80 h-2.5 w-2.5 rounded-full border transition-colors",
                        idx === currentPage ? "bg-foreground" : "bg-muted"
                      )}
                    />
                  ))}
                </nav>
                <span className="text-muted-foreground text-xs">
                  Page {currentPage + 1} of {pageCount}
                </span>
              </div>
            ) : null}
          </section>
        </div>

        <div className="border-border/80 bg-muted/40 text-muted-foreground grid grid-cols-2 gap-3 rounded-lg border px-3 py-2 text-xs sm:grid-cols-3">
          <div className="flex items-center gap-2">
            <KbdGroup>
              <Kbd>↑</Kbd>
              <Kbd>↓</Kbd>
            </KbdGroup>
            <span className="text-foreground/80">Select</span>
          </div>
          <div className="flex items-center gap-2">
            <KbdGroup>
              <Kbd>Enter</Kbd>
            </KbdGroup>
            <span className="text-foreground/80">Main action</span>
          </div>
          <div className="flex items-center gap-2">
            <KbdGroup>
              <Kbd>Ctrl</Kbd>
              <span className="opacity-60">+</span>
              <Kbd>O</Kbd>
            </KbdGroup>
            <span className="text-foreground/80">Optional actions</span>
          </div>
        </div>
        {isActionsOpen && dialogItem ? (
          <div className="bg-background/70 fixed inset-0 z-50 flex items-center justify-center backdrop-blur">
            <div
              role="dialog"
              aria-modal="true"
              aria-label={`Actions for ${dialogItem.Name}`}
              className="border-border/80 bg-card text-card-foreground w-full max-w-md rounded-lg border shadow-lg"
            >
              <header className="flex items-center justify-between gap-4 px-4 py-3">
                <div className="flex flex-col">
                  <span className="text-muted-foreground text-xs tracking-wide uppercase">
                    Optional actions
                  </span>
                  <span className="text-foreground font-mono text-sm">{dialogItem.Name}</span>
                </div>
              </header>
              <div className="text-muted-foreground space-y-2 px-4 pb-4 text-sm">
                {selectedActions.length ? (
                  <ul className="space-y-1">
                    {selectedActions.map((action) => (
                      <li
                        key={`${action.label}-${action.type}`}
                        className="text-foreground border-border/80 flex items-center justify-between rounded border px-2 py-1 text-xs"
                      >
                        <span className="font-medium">{action.label}</span>
                        <span className="text-muted-foreground tracking-wide uppercase">
                          {action.type}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p>No available actions. Press Escape to close.</p>
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </main>
  );
}

export default App;
