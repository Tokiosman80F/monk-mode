import type { AllowanceSite } from "./types/AllowanceSite";
import type { Lock } from "./types/Lock";
import type { Priority } from "./types/Priority";
import type { State } from "./types/State";
import type { Task } from "./types/Task";
import type { UsageDay } from "./types/UsageDay";

export const LOCK_PHRASE = "I commit to monk mode";

const BROWSER_NAMES: Record<string, string> = {
  "chrome.exe": "Chrome",
  "msedge.exe": "Edge",
  "brave.exe": "Brave",
};

export function browserName(process: string): string {
  return BROWSER_NAMES[process] ?? process;
}

export function formatDuration(totalSec: number): string {
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}

/** `8:05` or `1:02:03`. */
export function formatClock(totalSec: number): string {
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = String(totalSec % 60).padStart(2, "0");
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`;
  }
  return `${minutes}:${seconds}`;
}

export function siteUsedSec(state: State, domain: string): number {
  return state.today.site_used_sec[domain] ?? 0;
}

export function siteLeftSec(state: State, site: AllowanceSite): number {
  return Math.max(site.daily_min * 60 - siteUsedSec(state, site.domain), 0);
}

/** A ring's label: whole minutes, then a clock for the last minute. */
export function formatLeft(leftSec: number): string {
  if (leftSec === 0) return "Closed";
  return leftSec < 60 ? formatClock(leftSec) : formatDuration(leftSec);
}

export function totalFocusSec(day: UsageDay): number {
  return Object.values(day.focus_sec).reduce<number>(
    (sum, sec) => sum + (sec ?? 0),
    0,
  );
}

export const PRIORITIES: Priority[] = ["high", "medium", "low"];

/** `xp` must match `priority_xp` in crates/core/src/tasks.rs. */
export const PRIORITY_INFO: Record<
  Priority,
  { label: string; xp: number; tone: string }
> = {
  high: { label: "High", xp: 50, tone: "text-error" },
  medium: { label: "Medium", xp: 30, tone: "text-accent" },
  low: { label: "Low", xp: 15, tone: "text-neutral" },
};

export const SUBTASK_XP = 5;

export function nextPriority(priority: Priority): Priority {
  return PRIORITIES[(PRIORITIES.indexOf(priority) + 1) % PRIORITIES.length];
}

const MINUTES_PER_DAY = 24 * 60;
/** A bare number this size or under means hours, so "3" is 3 hours and "45" is 45 minutes. */
const BARE_HOURS_UP_TO = 12;

/** Minutes from text like `45m`, `3h`, `1h 30m`, `1.5h`, `1:30` or `3`. Null if it isn't a time. */
export function parseDuration(input: string): number | null {
  const text = input.trim().toLowerCase();
  const parts = /^(?:(\d+(?:\.\d+)?)\s*h[a-z]*)?\s*(?:(\d+)\s*m[a-z]*)?$/.exec(
    text,
  );
  const clock = /^(\d+):([0-5]\d)$/.exec(text);
  let minutes: number | null = null;
  if (text !== "" && parts) {
    minutes = Number(parts[1] ?? 0) * 60 + Number(parts[2] ?? 0);
  } else if (clock) {
    minutes = Number(clock[1]) * 60 + Number(clock[2]);
  } else if (/^\d+(\.\d+)?$/.test(text)) {
    const value = Number(text);
    minutes = value <= BARE_HOURS_UP_TO ? value * 60 : value;
  }
  if (minutes === null) return null;
  minutes = Math.round(minutes);
  return minutes > 0 && minutes <= MINUTES_PER_DAY ? minutes : null;
}

/** Time set aside in tasks that aren't finished yet. */
export function plannedMin(tasks: Task[]): number {
  return tasks
    .filter((task) => !task.done)
    .reduce((sum, task) => sum + task.allocated_min, 0);
}

/** Minutes the subtasks go over the task's own time. Zero if the task has no time set. */
export function subtasksOverBy(task: Task): number {
  if (task.allocated_min === 0) return 0;
  const steps = task.subtasks.reduce((sum, s) => sum + s.allocated_min, 0);
  return Math.max(steps - task.allocated_min, 0);
}

/** The task last focused on if it's still open, so a break doesn't move you off it. Otherwise the
 * most important open task. */
export function nextTask(
  tasks: Task[],
  lastFocus: string | null = null,
): Task | undefined {
  const open = sortTasks(tasks).filter((task) => !task.done);
  return open.find((task) => task.title === lastFocus) ?? open[0];
}

/** Must match `time_left` in app/src-tauri/src/notify.rs, which the tray uses. */
export function timeLeftText(seconds: number): string {
  return seconds < 60
    ? `${seconds} sec left`
    : `${Math.ceil(seconds / 60)} min left`;
}

/** The wall-clock time something `remainingSec` from `now` ends, like `10:50 AM`. */
export function endsAt(remainingSec: number, now = new Date()): string {
  return new Date(now.getTime() + remainingSec * 1000).toLocaleTimeString(
    undefined,
    { hour: "numeric", minute: "2-digit" },
  );
}

function nextId(items: { id: number }[]): number {
  return Math.max(0, ...items.map((item) => item.id)) + 1;
}

export function addTask(
  tasks: Task[],
  title: string,
  priority: Priority,
  allocated_min: number,
): Task[] {
  return [
    ...tasks,
    {
      id: nextId(tasks),
      title,
      priority,
      done: false,
      done_on: null,
      allocated_min,
      subtasks: [],
    },
  ];
}

/** Open tasks first, then by priority. Within that, oldest first. */
export function sortTasks(tasks: Task[]): Task[] {
  return [...tasks].sort(
    (a, b) =>
      Number(a.done) - Number(b.done) ||
      PRIORITIES.indexOf(a.priority) - PRIORITIES.indexOf(b.priority) ||
      a.id - b.id,
  );
}

/** A task with subtasks is done exactly when all of them are. */
export function toggleTask(task: Task): Task {
  const done = !task.done;
  return {
    ...task,
    done,
    subtasks: task.subtasks.map((subtask) => ({ ...subtask, done })),
  };
}

export function toggleSubtask(task: Task, id: number): Task {
  const subtasks = task.subtasks.map((subtask) =>
    subtask.id === id ? { ...subtask, done: !subtask.done } : subtask,
  );
  return { ...task, subtasks, done: subtasks.every((s) => s.done) };
}

export function addSubtask(task: Task, title: string, allocated_min = 0): Task {
  const subtask = {
    id: nextId(task.subtasks),
    title,
    done: false,
    allocated_min,
  };
  return { ...task, done: false, subtasks: [...task.subtasks, subtask] };
}

export function removeSubtask(task: Task, id: number): Task {
  const subtasks = task.subtasks.filter((subtask) => subtask.id !== id);
  const done = subtasks.length > 0 ? subtasks.every((s) => s.done) : task.done;
  return { ...task, subtasks, done };
}

export function isLocked(lock: Lock | null, now = Date.now()): boolean {
  return lock !== null && new Date(lock.ends_at).getTime() > now;
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function lockedUntil(lock: Lock | null): string {
  return lock ? `Locked until ${formatDate(lock.ends_at)}` : "";
}

/** Local mirror of the service's domain cleaning, so a pasted URL just works. */
export function cleanDomain(input: string): string {
  return input
    .trim()
    .toLowerCase()
    .replace(/^[a-z][a-z0-9+.-]*:\/\//, "")
    .split(/[/?#\\]/)[0]
    .split("@")
    .pop()!
    .split(":")[0]
    .replace(/\.+$/, "")
    .replace(/^www\./, "");
}

function dateKey(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

export interface ChartDay {
  date: string;
  focusSec: number;
}

/** One entry per day for the last `count` days ending at `end`, with gaps at zero. */
export function fillDays(
  days: UsageDay[],
  count: number,
  end: Date,
): ChartDay[] {
  const byDate = new Map(days.map((day) => [day.date, day]));
  return Array.from({ length: count }, (_, i) => {
    const date = new Date(end);
    date.setDate(end.getDate() - (count - 1 - i));
    const key = dateKey(date);
    const day = byDate.get(key);
    return { date: key, focusSec: day ? totalFocusSec(day) : 0 };
  });
}

export function addDays(from: Date, days: number): Date {
  const date = new Date(from);
  date.setDate(date.getDate() + days);
  return date;
}

export function addMonths(from: Date, months: number): Date {
  const date = new Date(from);
  date.setMonth(date.getMonth() + months);
  return date;
}

/**
 * The focus ring: one segment per focus session in the task's planned time.
 * `remainingSec` is null during a break.
 */
export function sessionRing(
  plannedMin: number,
  focusMin: number,
  spentSec: number,
  remainingSec: number | null,
): { total: number; done: number; current: number | null } {
  const length = Math.max(focusMin, 1) * 60;
  const inCurrent =
    remainingSec === null ? 0 : Math.max(0, length - remainingSec);
  const done = Math.max(0, Math.floor((spentSec - inCurrent) / length));
  const total = Math.max(
    Math.ceil(plannedMin / Math.max(focusMin, 1)),
    done + (remainingSec === null ? 0 : 1),
    1,
  );
  return {
    total,
    done,
    current: remainingSec === null ? null : inCurrent / length,
  };
}
