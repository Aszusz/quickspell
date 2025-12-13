import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StateSnapshot } from "./events";
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
import { ScrollArea } from "./components/ui/scroll-area";
import { Item, ItemGroup, ItemSeparator, ItemTitle } from "./components/ui/item";
import { Kbd, KbdGroup } from "./components/ui/kbd";

const DEFAULT_SNAPSHOT: StateSnapshot = {
  status: "loading",
  noOfSpells: 0,
  totalItems: 0,
  spellNames: [],
  topItems: [],
};

function App() {
  const [snapshot, setSnapshot] = useState<StateSnapshot>(DEFAULT_SNAPSHOT);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useOsTheme();

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

  const handleSearchBlur = () => {
    // Keep focus on the search box even after clicking outside.
    requestAnimationFrame(() => searchRef.current?.focus());
  };

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
            <Search className="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2" />
            <Input
              ref={searchRef}
              className="w-full pr-14 pl-10"
              placeholder="Type to search..."
              onBlur={handleSearchBlur}
            />
            <span className="text-muted-foreground pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs font-medium select-none">
              {snapshot.totalItems}
            </span>
          </div>

          <section className="flex min-h-0 flex-1 flex-col">
            {snapshot.topItems.length ? (
              <ScrollArea className="border-border/80 bg-muted/40 h-full w-full rounded-lg border">
                <ItemGroup>
                  {snapshot.topItems.map((item, idx) => (
                    <React.Fragment key={`${item}-${idx}`}>
                      <Item size="sm" variant="muted" className="rounded-none border-0 px-3 py-2">
                        <ItemTitle className="font-mono text-xs">{item}</ItemTitle>
                      </Item>
                      {idx < snapshot.topItems.length - 1 ? <ItemSeparator /> : null}
                    </React.Fragment>
                  ))}
                </ItemGroup>
              </ScrollArea>
            ) : (
              <div className="text-muted-foreground text-sm">No items loaded</div>
            )}
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
      </div>
    </main>
  );
}

export default App;
