import type { Lock } from "./types/Lock";
import type { State } from "./types/State";
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

export function allowanceLeftSec(state: State): number {
  const limit = state.settings.daily_allowance_min * 60;
  return Math.max(limit - state.today.allowance_used_sec, 0);
}

export function allowanceText(leftSec: number): string {
  if (leftSec === 0) return "Nothing left for today.";
  if (leftSec < 60) return `${leftSec} seconds left for today.`;
  const minutes = Math.floor(leftSec / 60);
  return `${minutes} ${minutes === 1 ? "minute" : "minutes"} left for today.`;
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
    const focus = byDate.get(key)?.focus_sec ?? {};
    const focusSec = Object.values(focus).reduce<number>(
      (sum, sec) => sum + (sec ?? 0),
      0,
    );
    return { date: key, focusSec };
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
