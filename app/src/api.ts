import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Settings } from "./types/Settings";
import type { SmallItem } from "./types/SmallItem";
import type { Snapshot } from "./types/Snapshot";
import type { Stats } from "./types/Stats";
import type { Task } from "./types/Task";

export const api = {
  getState: () => invoke<Snapshot>("get_state"),
  updateSettings: (settings: Settings) =>
    invoke<Snapshot>("update_settings", { settings }),
  startLock: (endsAt: string) => invoke<Snapshot>("start_lock", { endsAt }),
  updateTasks: (tasks: Task[]) => invoke<Snapshot>("update_tasks", { tasks }),
  updateSmallItems: (items: SmallItem[]) =>
    invoke<Snapshot>("update_small_items", { items }),
  startPomodoro: (taskId: number | null) =>
    invoke<Snapshot>("start_pomodoro", { taskId }),
  stopPomodoro: () => invoke<Snapshot>("stop_pomodoro"),
  pausePomodoro: (paused: boolean) =>
    invoke<Snapshot>("pause_pomodoro", { paused }),
  getStats: (days: number) => invoke<Stats>("get_stats", { days }),
  installService: () => invoke<void>("install_service"),
  openExtensionFolder: () => invoke<string>("open_extension_folder"),
  dismissMini: () => invoke<void>("dismiss_mini"),

  onStateChanged: (handler: (snapshot: Snapshot) => void) =>
    listen<Snapshot>("state-changed", (event) => handler(event.payload)),
  onServiceStatus: (handler: (connected: boolean) => void) =>
    listen<boolean>("service-status", (event) => handler(event.payload)),
};

/** The text of whatever a failed call threw. */
export function errorText(error: unknown): string {
  if (typeof error === "object" && error && "message" in error) {
    return String(error.message);
  }
  return String(error);
}
