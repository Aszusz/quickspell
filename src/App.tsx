import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { Input } from "./components/ui/input";
import { Label } from "./components/ui/label";
import { useOsTheme } from "./hooks/use-os-theme";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  useOsTheme();

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="bg-background flex min-h-screen justify-center p-8 pt-16">
      <div className="flex w-full max-w-md flex-col gap-4">
        <div className="space-y-1 text-center">
          <h1 className="text-2xl font-semibold tracking-tight">Zero To Tauri</h1>
          <p className="text-muted-foreground text-sm">
            Shadcn theme adapts to your OS preferences
          </p>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Greet</CardTitle>
            <CardDescription>Enter your name to receive a greeting</CardDescription>
          </CardHeader>
          <CardContent>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                greet();
              }}
            >
              <div className="grid gap-2">
                <Label htmlFor="greet-input">Name</Label>
                <Input
                  id="greet-input"
                  value={name}
                  onChange={(e) => setName(e.currentTarget.value)}
                  placeholder="Enter a name..."
                  required
                />
              </div>
            </form>
          </CardContent>
          <CardFooter className="flex-col gap-2">
            <Button type="submit" className="w-full" onClick={greet}>
              Greet
            </Button>
            <p className="text-muted-foreground min-h-5 text-center text-sm">{greetMsg}</p>
          </CardFooter>
        </Card>
      </div>
    </main>
  );
}

export default App;
