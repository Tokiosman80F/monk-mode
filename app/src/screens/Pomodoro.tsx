import { Play, Square } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { formatClock, formatDuration, nextTask } from "../lib";
import { applyCall, useStore } from "../store";
import { ErrorNote, Logo } from "../ui";

export default function Pomodoro() {
  const state = useStore((s) => s.snapshot?.state);
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const { pomodoro } = state;
  const next = nextTask(state.tasks);
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
                <span className="text-neutral">Next up: </span>
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
  const clock = formatClock(pomodoro.remaining_sec);
  return (
    <section
      className={`rounded-box border bg-base-200 p-5 ${focusing ? "border-secondary" : "border-base-300"}`}
    >
      <div className="flex flex-wrap items-center gap-5">
        {focusing && <Logo breathing className="w-12" />}
        <div className="min-w-0 flex-1">
          <h2 className="truncate font-display text-lg font-medium">
            {focusing ? `Focus: ${pomodoro.task_title}` : "Break"}
          </h2>
          <p className="text-sm text-neutral">
            {focusing
              ? "Sites with limits are closed until this ends. Each minute earns 1 XP."
              : "Back to work when you're ready."}
          </p>
        </div>
        <span
          role="timer"
          className={`font-display text-5xl font-light tabular-nums tracking-tight ${focusing ? "text-secondary" : ""}`}
        >
          {clock}
        </span>
        <button className="btn btn-ghost" onClick={() => void stop()}>
          <Square size={18} aria-hidden="true" />
          {focusing ? "Stop" : "Skip break"}
        </button>
      </div>
      {error && (
        <div className="mt-3">
          <ErrorNote message={error} />
        </div>
      )}
    </section>
  );
}
