import { describe, expect, it } from "vitest";
import {
  addMonths,
  addSubtask,
  addTask,
  browserName,
  cleanDomain,
  fillDays,
  formatClock,
  formatDuration,
  formatLeft,
  greeting,
  isLocked,
  PRIORITY_INFO,
  removeSubtask,
  siteLeftSec,
  sitesByUse,
  sortTasks,
  SUBTASK_XP,
  toggleSubtask,
  toggleTask,
} from "./lib";
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

  it("puts the sites in use first", () => {
    const { state } = makeSnapshot();
    state.today.site_used_sec = { "reddit.com": 5 };
    expect(sitesByUse(state)[0].domain).toBe("reddit.com");
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

  it("adds with the next free ID", () => {
    const added = addTask(tasks, "New", "low");
    expect(added[2]).toMatchObject({ id: 3, title: "New", priority: "low" });
    expect(addTask([], "First", "high")[0].id).toBe(1);
  });

  it("sorts open before done, then by priority, then by age", () => {
    const low = addTask(tasks, "Low", "low");
    const done = low.map((t) => (t.id === 2 ? { ...t, done: true } : t));
    expect(sortTasks(low).map((t) => t.id)).toEqual([2, 1, 3]);
    expect(sortTasks(done).map((t) => t.id)).toEqual([1, 3, 2]);
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
    expect(next.subtasks[0]).toEqual({ id: 1, title: "More", done: false });
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

it.each([
  [8, "Good morning"],
  [13, "Good afternoon"],
  [20, "Good evening"],
])("at %i it says %s", (hour, text) => expect(greeting(hour)).toBe(text));

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
