import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listenEvent } from "./events";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { useOsTheme } from "./hooks/use-os-theme";

function App() {
  const [count, setCount] = useState<number>(0);

  useOsTheme();

  useEffect(() => {
    invoke<number>("get_count").then(setCount);

    const unlisten = listenEvent("counter-changed", (payload) => {
      setCount(payload.count);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <main className="bg-background flex min-h-screen justify-center p-8 pt-16">
      <div className="flex w-full max-w-md flex-col gap-4">
        <div className="space-y-1 text-center">
          <h1 className="text-2xl font-semibold tracking-tight">Counter App</h1>
          <p className="text-muted-foreground text-sm">Rust-managed state with Tauri</p>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Counter</CardTitle>
            <CardDescription>State managed on Rust backend</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col items-center gap-4">
            <div className="text-6xl font-bold tabular-nums">{count}</div>
            <div className="flex gap-2">
              <Button variant="outline" size="lg" onClick={() => invoke("decrement")}>
                -
              </Button>
              <Button size="lg" onClick={() => invoke("increment")}>
                +
              </Button>
            </div>
            <Button variant="ghost" onClick={() => invoke("reset")}>
              Reset
            </Button>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

export default App;
