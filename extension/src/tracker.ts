import type { ActiveTab } from "./types/ActiveTab";

const NO_TAB: ActiveTab = {
  url: "",
  focused: false,
  visible: false,
  incognito: false,
  idle: false,
};

/**
 * `chrome.idle` only knows about mouse and keyboard input, so someone watching a video looks idle.
 * Sound playing means they're still there. Locked always means they're not.
 */
export function isIdle(
  idleState: chrome.idle.IdleState,
  audible: boolean,
): boolean {
  return idleState === "locked" || (idleState === "idle" && !audible);
}

/** What the user is looking at right now, in this browser. */
export async function currentTab(
  idleState: chrome.idle.IdleState,
): Promise<ActiveTab> {
  const [tab] = await chrome.tabs.query({
    active: true,
    lastFocusedWindow: true,
  });
  if (!tab?.url || tab.windowId === undefined) return NO_TAB;

  const window = await chrome.windows.get(tab.windowId);
  return {
    url: tab.url,
    focused: window.focused,
    visible: window.state !== "minimized",
    incognito: tab.incognito,
    idle: isIdle(idleState, tab.audible === true),
  };
}

/** The process name the service knows this browser by. */
export function browserProcess(nav: {
  userAgentData?: { brands: { brand: string }[] };
  brave?: unknown;
}): string {
  const brands = nav.userAgentData?.brands.map((b) => b.brand) ?? [];
  if (nav.brave || brands.includes("Brave")) return "brave.exe";
  if (brands.includes("Microsoft Edge")) return "msedge.exe";
  return "chrome.exe";
}
