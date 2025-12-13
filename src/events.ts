import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface CounterChanged {
  count: number;
}

interface EventMap {
  "counter-changed": CounterChanged;
}

export function listenEvent<K extends keyof EventMap>(
  event: K,
  handler: (payload: EventMap[K]) => void
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}
