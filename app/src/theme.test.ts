import { describe, expect, it } from "vitest";
import { monkDark as t } from "./theme";

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

describe("monk-dark", () => {
  it.each([
    ["base-content on base-100", t["base-content"], t["base-100"]],
    ["base-content on base-200", t["base-content"], t["base-200"]],
    ["base-content on base-300", t["base-content"], t["base-300"]],
    ["neutral on base-100", t.neutral, t["base-100"]],
    ["neutral on base-200", t.neutral, t["base-200"]],
    ["accent on base-200", t.accent, t["base-200"]],
    ["secondary on base-200", t.secondary, t["base-200"]],
    ["error on base-200", t.error, t["base-200"]],
    ["button label on primary", t["primary-content"], t.primary],
    ["label on accent", t["accent-content"], t.accent],
  ])("%s passes AA", (_, fg, bg) => {
    expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });
});
