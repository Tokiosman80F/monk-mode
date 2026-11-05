import { expect, it } from "vitest";
import { SHORTS_CSS, SHORTS_ENABLED_KEY, SHORTS_SELECTORS } from "./shorts";
import { SHORTS_KEY } from "./view";

it("hides with display none", () => {
  expect(SHORTS_CSS).toContain("display: none !important");
});

it("never targets a regular video element on its own", () => {
  const unconditional = [
    "ytd-rich-item-renderer",
    "ytd-video-renderer",
    "ytd-compact-video-renderer",
  ];
  for (const element of unconditional) {
    expect(SHORTS_SELECTORS).not.toContain(element);
  }
  for (const selector of SHORTS_SELECTORS.filter((s) =>
    unconditional.some((u) => s.startsWith(u)),
  )) {
    expect(selector).toContain("/shorts/");
  }
});

it("uses the same storage key as the background worker", () => {
  expect(SHORTS_ENABLED_KEY).toBe(SHORTS_KEY);
});
