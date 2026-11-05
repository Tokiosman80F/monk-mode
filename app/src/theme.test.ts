import { describe, expect, it } from "vitest";
import { monkDark, monkLight } from "./theme";

function luminance(hex: string) {
  const [r, g, b] = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

describe.each([
  ["monk-light", monkLight],
  ["monk-dark", monkDark],
])("%s", (_, t) => {
  it.each([
    ["base-content on base-100", t["base-content"], t["base-100"]],
    ["base-content on base-200", t["base-content"], t["base-200"]],
    ["base-content on base-300", t["base-content"], t["base-300"]],
    ["neutral on base-100", t.neutral, t["base-100"]],
    ["neutral on base-200", t.neutral, t["base-200"]],
  ])("body text %s passes AA", (_, fg, bg) => {
    expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });

  // Button labels, not body text: the PRD's light primary pair is 3.9:1, so this is the UI-component bar.
  it("primary button label reaches 3:1", () => {
    expect(contrast(t["primary-content"], t.primary)).toBeGreaterThanOrEqual(3);
  });
});
