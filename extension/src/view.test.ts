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

it("topTasks leaves out finished ones and puts high priority first", () => {
  const task = (
    id: number,
    priority: Task["priority"],
    done = false,
  ): Task => ({
    id,
    title: String(id),
    priority,
    done,
    done_on: null,
    subtasks: [],
  });
  const tasks = [
    task(1, "low"),
    task(2, "high", true),
    task(3, "high"),
    task(4, "medium"),
  ];
  expect(topTasks(tasks, 2).map((t) => t.id)).toEqual([3, 4]);
  expect(topTasks(tasks, 9)).toHaveLength(3);
});
