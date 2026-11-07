import { describe, expect, it } from "vitest";
import {
  addMonths,
  timeLeftText,
  endsAt,
  addSubtask,
  addTask,
  browserName,
  cleanDomain,
  fillDays,
  formatClock,
  formatDuration,
  formatLeft,
  isLocked,
  nextPriority,
  nextTask,
  parseDuration,
  plannedMin,
  PRIORITY_INFO,
  removeSubtask,
  sessionRing,
  siteLeftSec,
  sortTasks,
  subtasksOverBy,
  SUBTASK_XP,
  toggleSubtask,
  toggleTask,
} from "./lib";
import { quoteFor } from "./quotes";
import tasksRs from "../../crates/core/src/tasks.rs?raw";
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

describe("site limits", () => {
  it("counts each site on its own and stops at zero", () => {
    const { state } = makeSnapshot();
    const [youtube, reddit] = state.settings.allowance_sites;
    expect(siteLeftSec(state, youtube)).toBe(1680);
    expect(siteLeftSec(state, reddit)).toBe(600);
    state.today.site_used_sec["reddit.com"] = 720;
    expect(siteLeftSec(state, reddit)).toBe(0);
  });

  it.each([
    [0, "Closed"],
    [45, "0:45"],
    [60, "1m"],
    [1680, "28m"],
    [5400, "1h 30m"],
  ])("%i reads as %s", (sec, text) => expect(formatLeft(sec)).toBe(text));
});

describe("tasks", () => {
  const { tasks } = makeSnapshot().state;
  const [plain, withSteps] = tasks;

  it("adds with the next free ID and its time", () => {
    const added = addTask(tasks, "New", "low", 45);
    expect(added[2]).toMatchObject({
      id: 3,
      title: "New",
      priority: "low",
      allocated_min: 45,
    });
    expect(addTask([], "First", "high", 5)[0].id).toBe(1);
  });

  it("sorts open before done, then by priority and age", () => {
    const more = addTask(tasks, "Low", "low", 5);
    const done = more.map((t) => (t.id === 2 ? { ...t, done: true } : t));
    expect(sortTasks(more).map((t) => t.id)).toEqual([2, 1, 3]);
    expect(sortTasks(done).map((t) => t.id)).toEqual([1, 3, 2]);
  });

  it("next up skips finished tasks", () => {
    expect(nextTask(tasks)?.id).toBe(2);
    expect(nextTask([{ ...plain, done: true }])).toBeUndefined();
  });

  it("next up stays on the task last focused on while it's open", () => {
    expect(nextTask(tasks, "Learn hooks")?.id).toBe(1);
    expect(
      nextTask([{ ...plain, done: true }, withSteps], "Learn hooks")?.id,
    ).toBe(2);
    expect(nextTask(tasks, "Free focus")?.id).toBe(2);
  });

  it.each([
    [49 * 60, "49 min left"],
    [49 * 60 - 30, "49 min left"],
    [60, "1 min left"],
    [59, "59 sec left"],
    [0, "0 sec left"],
  ])("%i seconds reads as %s", (sec, text) => {
    expect(timeLeftText(sec)).toBe(text);
  });

  it("works out when a session ends", () => {
    const tenAm = new Date(2026, 10, 6, 10, 0, 0);
    const expected = new Date(2026, 10, 6, 10, 50).toLocaleTimeString(
      undefined,
      { hour: "numeric", minute: "2-digit" },
    );
    expect(endsAt(50 * 60, tenAm)).toBe(expected);
  });

  it("adds up open time and warns when subtasks go over", () => {
    expect(plannedMin(tasks)).toBe(270);
    expect(plannedMin([{ ...plain, done: true }])).toBe(0);
    expect(subtasksOverBy(withSteps)).toBe(0);
    expect(subtasksOverBy({ ...withSteps, allocated_min: 20 })).toBe(10);
    expect(subtasksOverBy({ ...withSteps, allocated_min: 0 })).toBe(0);
  });

  it("finishing a task finishes its subtasks, and undoing undoes them", () => {
    const done = toggleTask(withSteps);
    expect(done.done).toBe(true);
    expect(done.subtasks.every((s) => s.done)).toBe(true);
    const undone = toggleTask(done);
    expect(undone.done).toBe(false);
    expect(undone.subtasks.some((s) => s.done)).toBe(false);
  });

  it("a task with subtasks is done exactly when they all are", () => {
    const finished = toggleSubtask(withSteps, 2);
    expect(finished.done).toBe(true);
    expect(toggleSubtask(finished, 1).done).toBe(false);
  });

  it("a new subtask reopens a finished task", () => {
    const next = addSubtask(toggleTask(plain), "More");
    expect(next.done).toBe(false);
    expect(next.subtasks[0]).toEqual({
      id: 1,
      title: "More",
      done: false,
      allocated_min: 0,
    });
    expect(addSubtask(plain, "Timed", 20).subtasks[0].allocated_min).toBe(20);
    expect(addSubtask(withSteps, "More").subtasks[2].id).toBe(3);
  });

  it("removing the last open subtask finishes the task", () => {
    expect(removeSubtask(withSteps, 2).done).toBe(true);
    expect(removeSubtask(removeSubtask(withSteps, 2), 1).subtasks).toEqual([]);
  });

  it("shows the same XP the service awards", () => {
    for (const [name, value] of [
      ["High", PRIORITY_INFO.high.xp],
      ["Medium", PRIORITY_INFO.medium.xp],
      ["Low", PRIORITY_INFO.low.xp],
    ]) {
      expect(tasksRs).toContain(`Priority::${name} => ${value}`);
    }
    expect(tasksRs).toContain(`SUBTASK_XP: u32 = ${SUBTASK_XP}`);
  });
});

describe("parseDuration", () => {
  it.each([
    ["45m", 45],
    ["45 min", 45],
    ["3h", 180],
    ["3 hours", 180],
    ["1h 30m", 90],
    ["1h30", null],
    ["1.5h", 90],
    ["1:30", 90],
    ["3", 180],
    ["1.5", 90],
    ["45", 45],
    ["24h", 1440],
    ["25h", null],
    ["0", null],
    ["", null],
    ["soon", null],
    ["-1h", null],
  ])("%s is %s", (text, minutes) => {
    expect(parseDuration(text)).toBe(minutes);
  });
});

describe("priority", () => {
  it("cycles", () => {
    expect(nextPriority("high")).toBe("medium");
    expect(nextPriority("low")).toBe("high");
  });
});

describe("quoteFor", () => {
  it("stays the same all day and changes the next", () => {
    const morning = quoteFor(new Date(2026, 10, 5, 6));
    expect(quoteFor(new Date(2026, 10, 5, 23))).toEqual(morning);
    expect(quoteFor(new Date(2026, 10, 6, 6))).not.toEqual(morning);
  });
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
        site_used_sec: {},
        task_xp: 0,
        tasks_done: 0,
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

describe("sessionRing", () => {
  it("splits a task's planned time into focus sessions", () => {
    // 10 hours of 50 minute sessions, two done, 20 minutes into the third.
    expect(sessionRing(600, 50, 2 * 3000 + 1200, 1800)).toEqual({
      total: 12,
      done: 2,
      current: 0.4,
    });
    expect(sessionRing(600, 50, 3 * 3000, null)).toEqual({
      total: 12,
      done: 3,
      current: null,
    });
  });

  it("grows past the plan and has one segment with no plan", () => {
    expect(sessionRing(50, 50, 3000, 3000)).toEqual({
      total: 2,
      done: 1,
      current: 0,
    });
    expect(sessionRing(0, 50, 0, 3000)).toEqual({
      total: 1,
      done: 0,
      current: 0,
    });
  });
});
