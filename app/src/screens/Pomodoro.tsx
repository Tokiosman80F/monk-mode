import { Pause, Play, Square } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import {
  endsAt,
  formatClock,
  formatDuration,
  nextTask,
  sessionRing,
} from "../lib";
import { applyCall, lastFocusTask, useStore } from "../store";
import { ErrorNote } from "../ui";

/** An arc on a 240 by 240 ring of radius 100, angles in turns from the top. */
function arc(from: number, to: number): string {
  const point = (turn: number) => {
    const a = turn * 2 * Math.PI;
    return `${(120 + 100 * Math.sin(a)).toFixed(2)} ${(120 - 100 * Math.cos(a)).toFixed(2)}`;
  };
  return `M${point(from)} A100 100 0 ${to - from > 0.5 ? 1 : 0} 1 ${point(to)}`;
}

function Ring({
  total,
  done,
  current,
  paused,
}: {
  total: number;
  done: number;
  current: number | null;
  paused: boolean;
}) {
  const gap = total > 1 ? Math.min(0.03, 0.3 / total) : 0;
  const size = 1 / total;
  return (
    <svg viewBox="0 0 240 240" className="h-full w-full" aria-hidden="true">
      {Array.from({ length: total }, (_, i) => {
        const from = i * size + gap / 2;
        const to = from + size - gap;
        // A full circle can't be one arc, so a single segment stops just short.
        const end = total === 1 ? to - 0.0001 : to;
        return (
          <g key={i} fill="none" strokeWidth="10" strokeLinecap="round">
            <path
              d={arc(from, end)}
              className={i < done ? "stroke-secondary" : "stroke-base-300"}
            />
            {i === done && current !== null && current > 0 && (
              <path
                d={arc(from, from + (end - from) * Math.min(current, 1))}
                className={paused ? "stroke-neutral" : "stroke-secondary"}
              />
            )}
          </g>
        );
      })}
    </svg>
  );
}

export default function Pomodoro() {
  const state = useStore((s) => s.snapshot?.state);
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const { pomodoro } = state;
  const next = nextTask(state.tasks, lastFocusTask());
  const carryingOn = next !== undefined && next.title === lastFocusTask();
  const stop = async () => setError(await applyCall(api.stopPomodoro()));
  const start = async (taskId: number | null) =>
    setError(await applyCall(api.startPomodoro(taskId)));

  if (pomodoro.phase === "idle") {
    return (
      <section className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <p className="min-w-40 flex-1 truncate">
            {next ? (
              <>
                <span className="text-neutral">
                  {carryingOn ? "Carry on with: " : "Next up: "}
                </span>
                {next.title}
                {next.allocated_min > 0 && (
                  <span className="text-neutral">
                    {" "}
                    for {formatDuration(next.allocated_min * 60)}
                  </span>
                )}
              </>
            ) : (
              <span className="text-neutral">Nothing lined up yet.</span>
            )}
          </p>
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => void start(null)}
          >
            Free focus
          </button>
          <button
            className="btn btn-primary btn-sm"
            disabled={!next}
            onClick={() => next && void start(next.id)}
          >
            <Play size={16} aria-hidden="true" />
            Start focus
          </button>
        </div>
        <ErrorNote message={error} />
      </section>
    );
  }

  const focusing = pomodoro.phase === "focus";
  const paused = focusing && pomodoro.paused;
  const title = focusing ? pomodoro.task_title : lastFocusTask();
  const task = state.tasks.find((t) => t.title === title);
  const ring = sessionRing(
    task?.allocated_min ?? 0,
    state.settings.pomodoro_focus_min,
    title ? (state.today.focus_sec[title] ?? 0) : 0,
    focusing ? pomodoro.remaining_sec : null,
  );
  const pause = async () =>
    setError(await applyCall(api.pausePomodoro(!paused)));

  return (
    <section
      className={`flex flex-col items-center gap-4 rounded-box border bg-base-200 p-5 ${focusing && !paused ? "border-secondary" : "border-base-300"}`}
    >
      <div className="flex w-full items-center justify-between gap-3">
        <h2 className="truncate font-display text-lg font-medium">
          {focusing ? `Focus: ${pomodoro.task_title}` : "Break"}
        </h2>
        <span className={`text-sm ${paused ? "text-warning" : "text-neutral"}`}>
          {paused ? "Paused" : focusing ? "Sites with limits closed" : "Rest"}
        </span>
      </div>
      <div className="relative size-60">
        <Ring {...ring} paused={paused} />
        <div className="absolute inset-0 flex flex-col items-center justify-center text-center">
          <span
            role="timer"
            className={`font-display text-5xl font-light tabular-nums tracking-tight ${paused ? "text-neutral" : ""}`}
          >
            {formatClock(pomodoro.remaining_sec)}
          </span>
          <span className="mt-1 text-sm text-neutral">
            {focusing
              ? `Session ${ring.done + 1} of ${ring.total}`
              : `${ring.done} of ${ring.total} sessions done`}
          </span>
          <span
            className={`text-sm ${paused ? "text-warning" : "text-secondary"}`}
          >
            {paused ? "Paused" : `Ends at ${endsAt(pomodoro.remaining_sec)}`}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-3">
        {focusing && (
          <button
            className="btn btn-circle btn-secondary"
            aria-label={paused ? "Resume" : "Pause"}
            onClick={() => void pause()}
          >
            {paused ? (
              <Play size={20} aria-hidden="true" />
            ) : (
              <Pause size={20} aria-hidden="true" />
            )}
          </button>
        )}
        <button className="btn btn-ghost btn-sm" onClick={() => void stop()}>
          <Square size={14} aria-hidden="true" />
          {focusing ? "Stop" : "Skip break"}
        </button>
      </div>
      <ErrorNote message={error} />
    </section>
  );
}
