import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AppStatus = "booting" | "loading" | "ready" | "error";

export interface StateSnapshot {
  status: AppStatus;
  noOfSpells: number;
  spellNames: string[];
}

interface EventMap {
  "state-snapshot": StateSnapshot;
}

export function listenEvent<K extends keyof EventMap>(
  event: K,
  handler: (payload: EventMap[K]) => void
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}
