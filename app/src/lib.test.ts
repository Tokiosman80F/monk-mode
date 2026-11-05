import { describe, expect, it } from "vitest";
import {
  addMonths,
  allowanceLeftSec,
  allowanceText,
  browserName,
  cleanDomain,
  fillDays,
  formatClock,
  formatDuration,
  isLocked,
} from "./lib";
import { makeSnapshot } from "./test/fixtures";

describe("formatDuration", () => {
  it.each([
    [0, "0m"],
    [6000, "1h 40m"],
    [10800, "3h"],
  ])("%i is %s", (sec, text) => expect(formatDuration(sec)).toBe(text));
});

describe("formatClock", () => {
  it.each([
    [0, "0:00"],
    [65, "1:05"],
    [600, "10:00"],
    [3723, "1:02:03"],
  ])("%i is %s", (sec, text) => expect(formatClock(sec)).toBe(text));
});

describe("allowance", () => {
  it("counts down and stops at zero", () => {
    const { state } = makeSnapshot();
    expect(allowanceLeftSec(state)).toBe(480);
    state.today.allowance_used_sec = 720;
    expect(allowanceLeftSec(state)).toBe(0);
  });

  it.each([
    [0, "Nothing left for today."],
    [45, "45 seconds left for today."],
    [60, "1 minute left for today."],
    [480, "8 minutes left for today."],
  ])("%i reads as %s", (sec, text) => expect(allowanceText(sec)).toBe(text));
});

describe("isLocked", () => {
  const lock = {
    created_at: "2026-11-01T00:00:00Z",
    ends_at: "2026-12-01T00:00:00Z",
  };

  it("is true only before the end", () => {
    expect(isLocked(null)).toBe(false);
    expect(isLocked(lock, Date.UTC(2026, 10, 15))).toBe(true);
    expect(isLocked(lock, Date.UTC(2026, 11, 1))).toBe(false);
  });
});

describe("cleanDomain", () => {
  it.each([
    ["https://www.YouTube.com/watch?v=1", "youtube.com"],
    ["  reddit.com  ", "reddit.com"],
    ["www.twitch.tv/x", "twitch.tv"],
    ["example.com:8080", "example.com"],
    ["open.spotify.com", "open.spotify.com"],
    ["https://a@b.com/", "b.com"],
    ["x.com.", "x.com"],
  ])("%s is %s", (input, output) => expect(cleanDomain(input)).toBe(output));
});

describe("fillDays", () => {
  const end = new Date(2026, 10, 5);

  it("fills gaps with zero and ends today", () => {
    const days = [
      {
        date: "2026-11-04",
        allowance_used_sec: 0,
        blocked_attempts: 0,
        focus_sec: { React: 60, Building: 40 },
      },
    ];
    expect(fillDays(days, 3, end)).toEqual([
      { date: "2026-11-03", focusSec: 0 },
      { date: "2026-11-04", focusSec: 100 },
      { date: "2026-11-05", focusSec: 0 },
    ]);
  });

  it("crosses month ends", () => {
    expect(fillDays([], 6, end)[0].date).toBe("2026-10-31");
  });
});

it("names browsers, and passes unknown ones through", () => {
  expect(browserName("msedge.exe")).toBe("Edge");
  expect(browserName("other.exe")).toBe("other.exe");
});

it("adds months", () => {
  expect(addMonths(new Date(2026, 10, 5), 3).getMonth()).toBe(1);
});
