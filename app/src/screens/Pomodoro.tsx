import { Play, Square } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { formatClock, sortTasks } from "../lib";
import { applyCall, useStore } from "../store";
import { ErrorNote, Logo } from "../ui";

const FREE_FOCUS = "";

export default function Pomodoro() {
  const state = useStore((s) => s.snapshot?.state);
  const [choice, setChoice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const open = sortTasks(state.tasks).filter((task) => !task.done);
  // Until something is picked, the most important open task.
  const picked = open.some((task) => String(task.id) === choice)
    ? choice
    : choice === FREE_FOCUS
      ? FREE_FOCUS
      : String(open[0]?.id ?? FREE_FOCUS);
  const { pomodoro } = state;

  const stop = async () => setError(await applyCall(api.stopPomodoro()));
  const start = async () =>
    setError(
      await applyCall(
        api.startPomodoro(picked === FREE_FOCUS ? null : Number(picked)),
      ),
    );

  if (pomodoro.phase === "idle") {
    return (
      <section className="rounded-box border border-base-300 bg-base-200 p-4">
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex min-w-48 flex-1 items-center gap-3">
            <span className="shrink-0 text-sm text-neutral">Focus on</span>
            <select
              className="select select-bordered min-w-0 flex-1"
              value={picked ?? FREE_FOCUS}
              onChange={(e) => setChoice(e.target.value)}
            >
              {open.map((task) => (
                <option key={task.id} value={task.id}>
                  {task.title}
                </option>
              ))}
              <option value={FREE_FOCUS}>Free focus (no task)</option>
            </select>
          </label>
          <button
            className="btn btn-primary shadow-glow"
            onClick={() => void start()}
          >
            <Play size={18} aria-hidden="true" />
            Start focus
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

  const focusing = pomodoro.phase === "focus";
  const clock = formatClock(pomodoro.remaining_sec);
  return (
    <section
      className={`rounded-box border p-6 ${
        focusing
          ? "border-primary bg-base-200 shadow-glow"
          : "border-base-300 bg-base-200"
      }`}
    >
      <div className="flex flex-wrap items-center gap-6">
        {focusing && <Logo breathing className="w-14" />}
        <div className="min-w-0 flex-1">
          <h2 className="truncate font-display text-lg font-semibold">
            {focusing ? `Focus: ${pomodoro.task_title}` : "Break"}
          </h2>
          <p className="text-sm text-neutral">
            {focusing
              ? "Sites with limits are closed until this ends. Every minute earns 1 XP."
              : "Back to work when you're ready."}
          </p>
        </div>
        <span
          key={clock}
          role="timer"
          className="font-display text-5xl font-semibold tabular-nums tracking-tight text-primary drop-shadow-glow"
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
