import { describe, expect, it } from "vitest";
import { formatDuration, streakText } from "./view";

describe("formatDuration", () => {
  it.each([
    [0, "0m"],
    [59, "0m"],
    [60, "1m"],
    [6000, "1h 40m"],
    [10800, "3h"],
  ])("%i seconds is %s", (sec, text) => {
    expect(formatDuration(sec)).toBe(text);
  });
});

describe("streakText", () => {
  it("handles singular and plural", () => {
    expect(streakText(1, null)).toBe("Streak: 1 day.");
    expect(streakText(0, null)).toBe("Streak: 0 days.");
  });

  it("includes the lock day when locked", () => {
    expect(streakText(4, { day: 12, total: 90 })).toBe(
      "Day 12 of 90. Streak: 4 days.",
    );
  });
});
