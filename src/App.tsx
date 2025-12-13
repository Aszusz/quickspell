import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StateSnapshot } from "./events";
import { listenEvent } from "./events";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { useOsTheme } from "./hooks/use-os-theme";

const DEFAULT_SNAPSHOT: StateSnapshot = {
  status: "booting",
  noOfSpells: 0,
  spellNames: [],
  topItems: [],
};

function App() {
  const [snapshot, setSnapshot] = useState<StateSnapshot>(DEFAULT_SNAPSHOT);

  useOsTheme();

  useEffect(() => {
    invoke<StateSnapshot>("get_state_snapshot").then(setSnapshot).catch(console.error);

    const unlisten = listenEvent("state-snapshot", (payload) => {
      setSnapshot(payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const statusLabel = useMemo(() => {
    switch (snapshot.status) {
      case "booting":
        return "Booting";
      case "loading":
        return "Loading spells";
      case "ready":
        return "Ready";
      case "error":
      default:
        return "Error";
    }
  }, [snapshot.status]);

  const spellNames = useMemo(
    () => [...snapshot.spellNames].sort((a, b) => a.localeCompare(b)),
    [snapshot.spellNames]
  );

  return (
    <main className="bg-background flex min-h-screen justify-center p-8 pt-16">
      <div className="flex w-full max-w-md flex-col gap-4">
        <div className="space-y-1 text-center">
          <h1 className="text-2xl font-semibold tracking-tight">QuickSpell</h1>
          <p className="text-muted-foreground text-sm">Tauri-powered launcher status</p>
        </div>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <div>
              <CardTitle>Backend State</CardTitle>
              <CardDescription>Live snapshot from the Rust core</CardDescription>
            </div>
            <span className="bg-muted text-foreground rounded-full px-3 py-1 text-xs font-semibold uppercase">
              {statusLabel}
            </span>
          </CardHeader>
          <CardContent className="flex flex-col gap-6">
            <div className="space-y-1">
              <div className="text-muted-foreground text-sm">Loaded spells</div>
              <div className="text-5xl font-bold tabular-nums">{snapshot.noOfSpells}</div>
            </div>
            <div className="space-y-2">
              <div className="text-muted-foreground text-sm">Top items</div>
              {snapshot.topItems.length ? (
                <ul className="bg-muted text-foreground divide-border divide-y overflow-hidden rounded-md">
                  {snapshot.topItems.map((item, idx) => (
                    <li key={`${item}-${idx}`} className="px-3 py-2 font-mono text-xs">
                      {item}
                    </li>
                  ))}
                </ul>
              ) : (
                <div className="text-muted-foreground text-sm">No items loaded</div>
              )}
            </div>
            <div className="space-y-2">
              <div className="text-muted-foreground text-sm">Spell names</div>
              {spellNames.length ? (
                <div className="flex flex-wrap gap-2">
                  {spellNames.map((name) => (
                    <span
                      key={name}
                      className="bg-muted text-foreground rounded-full px-3 py-1 text-xs font-semibold uppercase"
                    >
                      {name}
                    </span>
                  ))}
                </div>
              ) : (
                <div className="text-muted-foreground text-sm">No spells loaded</div>
              )}
            </div>
            <div className="text-muted-foreground text-sm">
              Status: <span className="text-foreground font-medium">{snapshot.status}</span>
            </div>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

export default App;
