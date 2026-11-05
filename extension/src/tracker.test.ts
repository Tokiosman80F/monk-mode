import { afterEach, describe, expect, it, vi } from "vitest";
import { browserProcess, currentTab, isIdle } from "./tracker";

function mockChrome(
  tab: Partial<chrome.tabs.Tab> | undefined,
  window: Partial<chrome.windows.Window>,
) {
  vi.stubGlobal("chrome", {
    tabs: { query: vi.fn(async () => (tab ? [tab] : [])) },
    windows: { get: vi.fn(async () => window) },
  });
}

const youtube = {
  url: "https://www.youtube.com/watch?v=1",
  windowId: 1,
  incognito: false,
  audible: false,
};

afterEach(() => vi.unstubAllGlobals());

describe("currentTab", () => {
  it("reports a focused, visible tab", async () => {
    mockChrome(youtube, { focused: true, state: "normal" });
    expect(await currentTab("active")).toEqual({
      url: youtube.url,
      focused: true,
      visible: true,
      incognito: false,
      idle: false,
    });
  });

  it("reports when the browser window isn't focused", async () => {
    mockChrome(youtube, { focused: false, state: "normal" });
    expect((await currentTab("active")).focused).toBe(false);
  });

  it("reports a minimized window as not visible", async () => {
    mockChrome(youtube, { focused: false, state: "minimized" });
    expect((await currentTab("active")).visible).toBe(false);
  });

  it("reports incognito tabs", async () => {
    mockChrome(
      { ...youtube, incognito: true },
      { focused: true, state: "normal" },
    );
    expect((await currentTab("active")).incognito).toBe(true);
  });

  it("queries only the active tab of the last focused window", async () => {
    mockChrome(youtube, { focused: true, state: "normal" });
    await currentTab("active");
    expect(chrome.tabs.query).toHaveBeenCalledWith({
      active: true,
      lastFocusedWindow: true,
    });
  });

  it.each([[undefined], [{ windowId: 1 }]])(
    "reports nothing to count for %j",
    async (tab) => {
      mockChrome(tab, { focused: true, state: "normal" });
      const report = await currentTab("active");
      expect(report.focused).toBe(false);
      expect(report.url).toBe("");
    },
  );

  it("marks an idle PC, unless sound is playing", async () => {
    mockChrome(youtube, { focused: true, state: "normal" });
    expect((await currentTab("idle")).idle).toBe(true);
    mockChrome(
      { ...youtube, audible: true },
      { focused: true, state: "normal" },
    );
    expect((await currentTab("idle")).idle).toBe(false);
  });
});

describe("isIdle", () => {
  it.each([
    ["active", false, false],
    ["active", true, false],
    ["idle", false, true],
    ["idle", true, false],
    ["locked", false, true],
    ["locked", true, true],
  ] as const)("%s, audible %s is %s", (state, audible, expected) => {
    expect(isIdle(state, audible)).toBe(expected);
  });
});

describe("browserProcess", () => {
  const brands = (...names: string[]) => ({
    userAgentData: { brands: names.map((brand) => ({ brand })) },
  });

  it("recognizes each browser", () => {
    expect(browserProcess(brands("Chromium", "Google Chrome"))).toBe(
      "chrome.exe",
    );
    expect(browserProcess(brands("Chromium", "Microsoft Edge"))).toBe(
      "msedge.exe",
    );
    expect(browserProcess(brands("Chromium", "Brave"))).toBe("brave.exe");
    expect(browserProcess({ ...brands("Chromium"), brave: {} })).toBe(
      "brave.exe",
    );
  });

  it("falls back to Chrome", () => {
    expect(browserProcess({})).toBe("chrome.exe");
  });
});
