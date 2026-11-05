import {
  allowanceLeftSec,
  allowanceText,
  browserName,
  formatClock,
  formatDuration,
} from "../lib";
import { useStore } from "../store";
import { Card, PageTitle, ProgressRow } from "../ui";
import Pomodoro from "./Pomodoro";

export default function Today() {
  const snapshot = useStore((s) => s.snapshot);
  if (!snapshot) return <PageTitle>Today</PageTitle>;

  const { state, missing_browsers: missing } = snapshot;
  const left = allowanceLeftSec(state);
  const clock = formatClock(left);

  return (
    <>
      <PageTitle>Today</PageTitle>

      {missing.length > 0 && (
        <p role="status" className="alert alert-warning text-sm">
          {missing.map(browserName).join(" and ")}{" "}
          {missing.length === 1 ? "is" : "are"} open without the extension.
          Allowance sites are blocked there after 30 seconds.
        </p>
      )}

      <div className="stats border border-base-300 bg-base-200 shadow-sm">
        <div className="stat">
          <div className="stat-title text-neutral">Allowance left</div>
          <div className="stat-value font-serif font-medium text-primary">
            <span key={clock} className="inline-block animate-fade-in">
              {clock}
            </span>
          </div>
          <div className="stat-desc text-sm text-neutral">
            {allowanceText(left)}
          </div>
        </div>
        <div className="stat">
          <div className="stat-title text-neutral">Streak</div>
          <div className="stat-value font-serif font-medium">
            {state.streak_days} {state.streak_days === 1 ? "day" : "days"}
          </div>
          <div className="stat-desc text-sm text-neutral">
            {state.lock_day
              ? `Day ${state.lock_day.day} of ${state.lock_day.total}. Keep going.`
              : "No lock running."}
          </div>
        </div>
      </div>

      <Card title="Study plan">
        {state.settings.study_blocks.map((block) => {
          const focus = state.today.focus_sec[block.name] ?? 0;
          const target = block.target_min * 60;
          return (
            <ProgressRow
              key={block.name}
              label={block.name}
              detail={`${formatDuration(focus)} / ${formatDuration(target)}`}
              value={focus}
              max={target}
            />
          );
        })}
      </Card>

      <Pomodoro />
    </>
  );
}
