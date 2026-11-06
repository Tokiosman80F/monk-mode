import { getCurrentWindow } from "@tauri-apps/api/window";

/** True in the small timer window, which loads the same page as the main one. Must match
 * `LABEL` in app/src-tauri/src/mini.rs. */
export function isMiniWindow(): boolean {
  try {
    return getCurrentWindow().label === "mini";
  } catch {
    // Outside Tauri, as in tests and the browser preview.
    return false;
  }
}
