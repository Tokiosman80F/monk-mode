import { describe, expect, it } from "vitest";
import type { Task } from "./types/Task";
import { formatDuration, streakText, topTasks } from "./view";

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

it("topTasks puts the most pressing first and leaves out finished and dropped ones", () => {
  const task = (
    id: number,
    priority: Task["priority"],
    quadrant: Task["quadrant"],
    done = false,
  ): Task => ({
    id,
    title: String(id),
    priority,
    done,
    done_on: null,
    quadrant,
    allocated_min: 0,
    subtasks: [],
  });
  const tasks = [
    task(1, "high", "schedule"),
    task(2, "high", "do_first", true),
    task(3, "low", "do_first"),
    task(4, "medium", "schedule"),
    task(5, "high", "eliminate"),
  ];
  expect(topTasks(tasks, 2).map((t) => t.id)).toEqual([3, 1]);
  expect(topTasks(tasks, 9)).toHaveLength(3);
});
