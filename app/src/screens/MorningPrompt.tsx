import { ArrowRight, CalendarClock, Plus, Target, X } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import {
  MAX_TASKS,
  addDays,
  addTask,
  dateKey,
  formatDuration,
  fromDateKey,
  parseDuration,
} from "../lib";
import { applyCall, useStore } from "../store";
import type { LeftoverAction } from "../types/LeftoverAction";
import type { Task } from "../types/Task";
import { ErrorNote } from "../ui";
import SmallWork from "./SmallWork";

function greeting(now = new Date()): string {
  const hour = now.getHours();
  if (hour >= 6 && hour < 12) return "Good morning";
  if (hour >= 12 && hour < 18) return "Good afternoon";
  return "Good evening";
}

function TaskMeta({ task }: { task: Task }) {
  const parts = [
    task.focus_area,
    task.allocated_min > 0 && formatDuration(task.allocated_min * 60),
  ].filter(Boolean);
  if (parts.length === 0) return null;
  return (
    <span className="whitespace-nowrap text-sm text-neutral">
      {parts.join(" · ")}
    </span>
  );
}

function Leftover({
  task,
  tomorrow,
  onResolve,
}: {
  task: Task;
  tomorrow: string;
  onResolve: (action: LeftoverAction) => void;
}) {
  const [moveTo, setMoveTo] = useState<string | null>(null);
  return (
    <li className="flex flex-wrap items-center gap-2 border-t border-base-300 py-2.5 first:border-t-0">
      <span className="min-w-0 flex-1 truncate">{task.title}</span>
      <TaskMeta task={task} />
      {moveTo === null ? (
        <span className="flex gap-1">
          <button
            className="btn btn-outline btn-sm"
            aria-label={`Carry ${task.title} to today`}
            onClick={() => onResolve({ kind: "carry" })}
          >
            Do today
          </button>
          <button
            className="btn btn-ghost btn-sm"
            aria-label={`Move ${task.title} to another day`}
            onClick={() => setMoveTo(tomorrow)}
          >
            Another day
          </button>
          <button
            className="btn btn-ghost btn-sm text-neutral"
            aria-label={`Drop ${task.title}`}
            onClick={() => onResolve({ kind: "drop" })}
          >
            Drop
          </button>
        </span>
      ) : (
        <span className="flex items-center gap-1">
          <input
            type="date"
            className="input input-sm input-bordered"
            aria-label={`Day to move ${task.title} to`}
            min={tomorrow}
            value={moveTo}
            onChange={(e) => setMoveTo(e.target.value)}
          />
          <button
            className="btn btn-outline btn-sm"
            onClick={() => moveTo && onResolve({ kind: "move", date: moveTo })}
          >
            Move
          </button>
          <button
            className="btn btn-ghost btn-sm px-1"
            aria-label="Cancel moving"
            onClick={() => setMoveTo(null)}
          >
            <X size={14} aria-hidden="true" />
          </button>
        </span>
      )}
    </li>
  );
}

/** Full screen until today has a plan: at least one task, confirmed. */
export default function MorningPrompt() {
  const state = useStore((s) => s.snapshot?.state);
  const areas = state?.settings.profile?.focus_areas ?? [];
  const [title, setTitle] = useState("");
  const [area, setArea] = useState(areas[0]?.name ?? "");
  const [estimate, setEstimate] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const { tasks, leftovers } = state;
  const full = tasks.length >= MAX_TASKS;
  const tomorrow = dateKey(addDays(fromDateKey(state.today.date), 1));
  /** True if it saved. */
  const saveTasks = async (next: Task[]) => {
    const failed = await applyCall(api.updateTasks(next));
    setError(failed);
    return failed === null;
  };

  /** The new task count, or null if nothing was added. */
  const add = async (): Promise<number | null> => {
    const name = title.trim();
    if (!name) return null;
    if (full) {
      setError(
        `${MAX_TASKS} tasks is the most for a day. Put quick jobs in small work.`,
      );
      return null;
    }
    const minutes = estimate.trim() === "" ? 0 : parseDuration(estimate);
    if (minutes === null) {
      setError("Use a time like 45m, 3h or 1h 30m, or leave it empty.");
      return null;
    }
    const next = addTask(tasks, name, "medium", minutes, state.next_id, area);
    if (!(await saveTasks(next))) return null;
    setTitle("");
    setEstimate("");
    return next.length;
  };

  const start = async () => {
    // A task typed but not added yet still counts.
    let count = tasks.length;
    if (title.trim()) {
      const added = await add();
      if (added === null) return;
      count = added;
    }
    if (count === 0) {
      setError(
        "Write at least one task to start your day. Small work doesn't count.",
      );
      return;
    }
    setError(await applyCall(api.planDay()));
  };

  return (
    <main className="min-h-screen overflow-y-auto bg-base-100 text-base-content">
      <div className="mx-auto flex max-w-2xl animate-fade-in flex-col gap-6 px-6 py-10">
        <header className="flex flex-col gap-2">
          <p className="text-neutral">
            {greeting()}
            {state.settings.profile && `, ${state.settings.profile.name}`}
          </p>
          <h1 className="font-display text-4xl font-light tracking-tight">
            Plan your day
          </h1>
          <p className="text-neutral">
            {fromDateKey(state.today.date).toLocaleDateString(undefined, {
              weekday: "long",
              month: "long",
              day: "numeric",
            })}
          </p>
        </header>

        {state.today_target && (
          <p className="flex items-start gap-3 rounded-box border border-secondary/40 bg-secondary/10 p-4">
            <Target
              size={20}
              aria-hidden="true"
              className="mt-0.5 shrink-0 text-secondary"
            />
            <span>
              <span className="block text-sm text-neutral">
                Today in your monthly plan
              </span>
              {state.today_target}
            </span>
          </p>
        )}

        {leftovers.length > 0 && (
          <section className="flex flex-col gap-1">
            <h2 className="flex items-center gap-2 font-display text-lg font-medium">
              <CalendarClock size={18} aria-hidden="true" />
              Left from yesterday
            </h2>
            <p className="text-sm text-neutral">
              Do each one today, move it to another day, or drop it.
            </p>
            <ul>
              {leftovers.map((task) => (
                <Leftover
                  key={task.id}
                  task={task}
                  tomorrow={tomorrow}
                  onResolve={(action) =>
                    void applyCall(api.resolveLeftover(task.id, action)).then(
                      setError,
                    )
                  }
                />
              ))}
            </ul>
          </section>
        )}

        <section className="flex flex-col gap-3">
          <div className="flex items-baseline justify-between">
            <h2 className="font-display text-lg font-medium">
              Today&apos;s tasks
            </h2>
            <span className="text-sm text-neutral">
              {tasks.length} of {MAX_TASKS} tasks
            </span>
          </div>
          <p className="text-sm text-neutral">
            Up to {MAX_TASKS} a day, for the important work that really counts.
            Quick 5 to 10 minute jobs go in small work.
          </p>
          {tasks.length > 0 && (
            <ul>
              {tasks.map((task) => (
                <li
                  key={task.id}
                  className="flex items-center gap-2 border-t border-base-300 py-2.5 first:border-t-0"
                >
                  <span className="min-w-0 flex-1 truncate">{task.title}</span>
                  <TaskMeta task={task} />
                  <button
                    className="btn btn-ghost btn-xs px-1"
                    aria-label={`Remove ${task.title}`}
                    onClick={() =>
                      void saveTasks(tasks.filter((t) => t.id !== task.id))
                    }
                  >
                    <X size={14} aria-hidden="true" />
                  </button>
                </li>
              ))}
            </ul>
          )}
          {full ? (
            <p role="status" className="text-sm text-neutral">
              That&apos;s {MAX_TASKS} tasks, the most for a day.
            </p>
          ) : (
            <form
              className="flex flex-wrap gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                void add();
              }}
            >
              <input
                className="input input-bordered min-w-52 flex-1"
                placeholder="Finish the recipe app's search"
                aria-label="New task"
                maxLength={200}
                value={title}
                onChange={(e) => setTitle(e.target.value)}
              />
              {areas.length > 0 && (
                <select
                  className="select select-bordered"
                  aria-label="Focus area"
                  value={area}
                  onChange={(e) => setArea(e.target.value)}
                >
                  {areas.map((a) => (
                    <option key={a.name}>{a.name}</option>
                  ))}
                </select>
              )}
              <input
                className="input input-bordered w-28"
                placeholder="Time"
                aria-label="Time estimate, optional"
                value={estimate}
                onChange={(e) => setEstimate(e.target.value)}
              />
              <button
                className="btn btn-ghost"
                type="submit"
                aria-label="Add task"
              >
                <Plus size={18} aria-hidden="true" />
                Add
              </button>
            </form>
          )}
        </section>

        <SmallWork />

        <ErrorNote message={error} />
        <button
          className="btn btn-primary self-start"
          onClick={() => void start()}
        >
          Start my day
          <ArrowRight size={18} aria-hidden="true" />
        </button>
      </div>
    </main>
  );
}
