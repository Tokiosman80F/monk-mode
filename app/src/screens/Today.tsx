import { Flame } from "lucide-react";
import { browserName, formatDuration, plannedMin, totalFocusSec } from "../lib";
import { useStore } from "../store";
import { DayHeader, PageTitle } from "../ui";
import Pomodoro from "./Pomodoro";
import SmallWork from "./SmallWork";
import Tasks from "./Tasks";

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

      <Pomodoro />
      <Tasks />
      <SmallWork />
    </>
  );
}
