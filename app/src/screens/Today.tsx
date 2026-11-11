import { Flame } from "lucide-react";
import { browserName, formatDuration, plannedMin, totalFocusSec } from "../lib";
import { useStore } from "../store";
import { DayHeader, LevelBar, PageTitle } from "../ui";
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
        <div className="flex flex-col gap-3">
          <LevelBar progress={state.progress} />
          <div className="flex items-center gap-2 text-sm">
            <Flame
              size={18}
              aria-hidden="true"
              className={streak > 0 ? "text-accent" : "text-neutral"}
            />
            <span>
              {streak} {streak === 1 ? "day" : "days"}
            </span>
            {lockDay && (
              <span className="text-neutral">
                Day {lockDay.day} of {lockDay.total}
              </span>
            )}
          </div>
          <div>
            <div className="mb-1 flex justify-between gap-4 text-xs text-neutral">
              <span>Focus today</span>
              <span>
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
