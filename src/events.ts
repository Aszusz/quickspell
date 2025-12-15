import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AppStatus = "notStarted" | "booting" | "loading" | "ready" | "error";

export interface Item {
  Type: string;
  Name: string;
  Data: string;
}

export type ActionType = "CMD" | "SPELL";

export interface AvailableAction {
  label: string;
  type: ActionType;
}

export interface SelectedItem {
  index: number;
  details: Item;
  actions: AvailableAction[];
}

export interface StateSnapshot {
  status: AppStatus;
  noOfSpells: number;
  totalItems: number;
  spellNames: string[];
  topItems: Item[];
  query: string;
  isFiltering: boolean;
  selectedItem: SelectedItem | null;
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
