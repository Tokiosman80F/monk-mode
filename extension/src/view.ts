import type { LockDay } from "./types/LockDay";
import type { Task } from "./types/Task";

const QUADRANT_ORDER = ["do_first", "schedule", "delegate"];
const PRIORITY_ORDER = ["high", "medium", "low"];

/** Where the background worker keeps the latest service state for the block page. */
export const STATE_KEY = "serviceState";

/** Must equal `SHORTS_ENABLED_KEY` in shorts.ts, which only the content script may import. */
export const SHORTS_KEY = "shortsBlockerEnabled";

export function formatDuration(totalSec: number): string {
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}

export function streakText(
  streakDays: number,
  lockDay: LockDay | null,
): string {
  const streak = `Streak: ${streakDays} ${streakDays === 1 ? "day" : "days"}.`;
  if (!lockDay) return streak;
  return `Day ${lockDay.day} of ${lockDay.total}. ${streak}`;
}

/** What the block page points back to: the most pressing open tasks, without any to drop. */
export function topTasks(tasks: Task[], count: number): Task[] {
  return tasks
    .filter((task) => !task.done && task.quadrant !== "eliminate")
    .sort(
      (a, b) =>
        QUADRANT_ORDER.indexOf(a.quadrant) -
          QUADRANT_ORDER.indexOf(b.quadrant) ||
        PRIORITY_ORDER.indexOf(a.priority) - PRIORITY_ORDER.indexOf(b.priority),
    )
    .slice(0, count);
}
