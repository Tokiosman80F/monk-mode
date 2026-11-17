import { Flame } from "lucide-react";
import {
  browserName,
  dayMinute,
  formatDuration,
  formatStart,
  plannedMin,
  totalFocusSec,
} from "../lib";
import type { Task } from "../types/Task";
import { useStore } from "../store";
import { DayHeader, PageTitle } from "../ui";
import Pomodoro from "./Pomodoro";
import SmallWork from "./SmallWork";
import Tasks from "./Tasks";

/** Today's time blocks on one strip, with a line for now. Hidden until a task has a time. */
function Timeline({
  tasks,
  current,
}: {
  tasks: Task[];
  current: number | null;
}) {
  const blocks = tasks.filter((t) => t.start_min !== null);
  if (blocks.length === 0) return null;
  const now = dayMinute();
  const starts = blocks.map((t) => t.start_min ?? 0);
  const ends = blocks.map((t) => (t.start_min ?? 0) + t.allocated_min);
  // Whole hours around the blocks and now, at least six hours wide.
  const from = Math.floor(Math.min(now, ...starts) / 60) * 60;
  const to = Math.max(Math.ceil(Math.max(now, ...ends) / 60) * 60, from + 360);
  const at = (minute: number) => `${((minute - from) / (to - from)) * 100}%`;
  const hours = Math.round((to - from) / 60);
  const step = hours > 12 ? 3 : hours > 6 ? 2 : 1;
  const marks = Array.from(
    { length: Math.floor(hours / step) + 1 },
    (_, i) => from + i * step * 60,
  );

  return (
    <section aria-label="Today's time blocks" className="flex flex-col gap-1">
      <div className="relative h-9 rounded-btn bg-base-200">
        {blocks.map((task) => (
          <div
            key={task.id}
            title={`${task.title}: ${formatStart(task.start_min ?? 0)} – ${formatStart((task.start_min ?? 0) + task.allocated_min)}`}
            className={`absolute inset-y-1 truncate rounded px-2 text-xs leading-7 ${
              task.done
                ? "bg-base-300 text-neutral line-through"
                : task.id === current
                  ? "bg-secondary font-medium text-secondary-content"
                  : "bg-base-300 text-base-content"
            }`}
            style={{
              left: at(task.start_min ?? 0),
              width: `calc(${at((task.start_min ?? 0) + task.allocated_min)} - ${at(task.start_min ?? 0)} - 2px)`,
            }}
          >
            {task.title}
          </div>
        ))}
        <div
          aria-hidden="true"
          className="absolute inset-y-0 w-0.5 bg-accent"
          style={{ left: at(now) }}
        />
      </div>
      <div className="relative h-4 text-[11px] text-neutral">
        {marks.map((minute) => (
          <span
            key={minute}
            className="absolute -translate-x-1/2 whitespace-nowrap first:translate-x-0 last:-translate-x-full"
            style={{ left: at(minute) }}
          >
            {formatStart(minute).replace(":00", "")}
          </span>
        ))}
      </div>
    </section>
  );
}

export default function Today() {
  const snapshot = useStore((s) => s.snapshot);
  if (!snapshot) return <PageTitle>Today</PageTitle>;

  const { state, missing_browsers: missing } = snapshot;
  const focusSec = totalFocusSec(state.today);
  const targetSec = state.settings.daily_focus_target_min * 60;
  const { streak_days: streak, lock_day: lockDay } = state;
  const planned = plannedMin(state.tasks);

  return (
    <>
      <header className="flex flex-wrap items-center justify-between gap-6">
        <DayHeader />
        {/* One progress bar: today's focus. Level and streak are one quiet line. */}
        <div className="flex w-64 flex-col gap-2">
          <div>
            <div className="mb-1 flex justify-between gap-4 text-sm">
              <span>Focus today</span>
              <span className="tabular-nums">
                {formatDuration(focusSec)} / {formatDuration(targetSec)}
              </span>
            </div>
            <progress
              className="progress progress-secondary h-1.5 w-full"
              aria-label="Focus today"
              value={focusSec}
              max={Math.max(targetSec, 1)}
            />
            <p className="mt-1 text-xs text-neutral">
              {formatDuration(planned * 60)} planned in open tasks
            </p>
          </div>
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-neutral">
            <span className="flex items-center gap-1">
              <Flame
                size={14}
                aria-hidden="true"
                className={streak > 0 ? "text-accent" : ""}
              />
              {streak}-day streak
            </span>
            <a href="#/rewards" className="hover:text-base-content">
              Level {state.progress.level}
            </a>
            {lockDay && (
              <span>
                Lock day {lockDay.day} of {lockDay.total}
              </span>
            )}
          </p>
        </div>
      </header>

      {missing.length > 0 && (
        <p role="status" className="alert alert-warning text-sm">
          {missing.map(browserName).join(" and ")}{" "}
          {missing.length === 1 ? "is" : "are"} open without the extension.
          Sites with limits are blocked there after 30 seconds.
        </p>
      )}

      <Timeline tasks={state.tasks} current={state.current_block} />
      <Pomodoro />
      <Tasks />
      <SmallWork />
    </>
  );
}
