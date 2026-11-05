import { describe, expect, it } from "vitest";
import manifest from "../public/manifest.json";
import engine from "../../crates/core/src/engine.rs?raw";
import {
  DEFAULT_SHORTS_PATTERNS,
  buildRules,
  isBlocked,
  matchesPattern,
} from "./rules";

describe("buildRules", () => {
  it("makes one main-frame redirect rule per pattern with unique IDs", () => {
    const rules = buildRules(["||tiktok.com^", "||youtube.com/shorts/"]);
    expect(rules.map((r) => r.id)).toEqual([1, 2]);
    expect(rules[1].condition.urlFilter).toBe("||youtube.com/shorts/");
    expect(rules[0].condition.resourceTypes).toEqual(["main_frame"]);
    expect(rules[0].action.redirect?.extensionPath).toBe("/block.html");
  });

  it("returns nothing for no patterns", () => {
    expect(buildRules([])).toEqual([]);
  });
});

describe("matchesPattern", () => {
  it.each([
    ["https://www.youtube.com/shorts/abc", "||youtube.com/shorts/", true],
    ["https://m.youtube.com/shorts/abc", "||youtube.com/shorts/", true],
    ["https://www.youtube.com/watch?v=1", "||youtube.com/shorts/", false],
    ["https://www.youtube.com/", "||youtube.com/shorts/", false],
    ["https://www.instagram.com/reel/x/", "||instagram.com/reel/", true],
    ["https://www.facebook.com/watch/?v=1", "||facebook.com/watch", true],
    ["https://www.tiktok.com/@a", "||tiktok.com^", true],
    ["https://tiktok.com/", "||tiktok.com^", true],
    ["https://nottiktok.com/", "||tiktok.com^", false],
    ["https://tiktok.com.evil.net/", "||tiktok.com^", false],
    ["https://evil.net/tiktok.com", "||tiktok.com^", false],
    ["chrome://extensions", "||tiktok.com^", false],
    ["not a url", "||tiktok.com^", false],
  ])("%s with %s is %s", (url, pattern, expected) => {
    expect(matchesPattern(url, pattern)).toBe(expected);
  });
});

it("isBlocked checks every pattern", () => {
  const patterns = ["||tiktok.com^", "||youtube.com/shorts/"];
  expect(isBlocked("https://youtube.com/shorts/1", patterns)).toBe(true);
  expect(isBlocked("https://react.dev/", patterns)).toBe(false);
  expect(isBlocked("https://react.dev/", [])).toBe(false);
});

describe("default rules", () => {
  it("match the service's Shorts patterns", () => {
    for (const pattern of DEFAULT_SHORTS_PATTERNS) {
      expect(engine).toContain(`"${pattern}"`);
    }
    expect(engine.match(/\[&str; (\d+)\] = \[\s*"\|\|youtube/)?.[1]).toBe(
      String(DEFAULT_SHORTS_PATTERNS.length),
    );
  });

  it("block Shorts but not normal videos", () => {
    expect(
      isBlocked("https://www.youtube.com/shorts/x", DEFAULT_SHORTS_PATTERNS),
    ).toBe(true);
    expect(
      isBlocked("https://www.youtube.com/watch?v=x", DEFAULT_SHORTS_PATTERNS),
    ).toBe(false);
  });
});

// Browsers compile a static ruleset into the extension's folder. The installed folder is
// read-only, so declaring one stops the extension loading at all.
it("declares no static rulesets", () => {
  expect(manifest).not.toHaveProperty("declarative_net_request");
});
