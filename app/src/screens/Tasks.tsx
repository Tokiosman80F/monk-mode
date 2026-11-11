import { ChevronRight, Pencil, Play, Plus, Trash2, X } from "lucide-react";
import { useRef, useState } from "react";
import { api } from "../api";
import {
  PRIORITIES,
  MAX_TASKS,
  PRIORITY_INFO,
  SUBTASK_XP,
  addSubtask,
  addTask,
  formatDuration,
  nextPriority,
  parseDuration,
  removeSubtask,
  sortTasks,
  subtasksOverBy,
  toggleSubtask,
  toggleTask,
} from "../lib";
import { applyCall, useStore } from "../store";
import type { Priority } from "../types/Priority";
import type { Task } from "../types/Task";
import { Card, ErrorNote } from "../ui";

const TIME_HINT = "Use a time like 45m, 3h or 1h 30m.";

function PriorityChip({
  value,
  label,
  onChange,
}: {
  value: Priority;
  label: string;
  onChange: (priority: Priority) => void;
}) {
  const info = PRIORITY_INFO[value];
  return (
    <button
      type="button"
      className={`btn btn-ghost btn-xs w-16 font-medium ${info.tone}`}
      aria-label={`${label}: ${info.label}. Change it`}
      onClick={() => onChange(nextPriority(value))}
    >
      {info.label}
    </button>
  );
}

/** Shows the time set aside, and becomes a field when clicked. */
function TimePill({
  minutes,
  spentSec = 0,
  label,
  onChange,
}: {
  minutes: number;
  spentSec?: number;
  label: string;
  onChange: (minutes: number) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);

  if (draft === null) {
    const planned = formatDuration(minutes * 60);
    const text =
      minutes === 0
        ? "Set time"
        : spentSec >= 60
          ? `${formatDuration(spentSec)} / ${planned}`
          : planned;
    return (
      <button
        type="button"
        className={`btn btn-ghost btn-xs w-24 justify-end whitespace-nowrap font-normal tabular-nums ${minutes === 0 ? "text-neutral" : ""}`}
        aria-label={`${label}: ${text}. Change it`}
        onClick={() => setDraft(minutes === 0 ? "" : planned)}
      >
        {text}
      </button>
    );
  }

  const commit = () => {
    const parsed = draft.trim() === "" ? 0 : parseDuration(draft);
    if (parsed !== null && parsed !== minutes) onChange(parsed);
    setDraft(null);
  };
  return (
    <input
      autoFocus
      className="input input-xs input-bordered w-24"
      aria-label={label}
      placeholder="1h 30m"
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") setDraft(null);
      }}
    />
  );
}

/** A task or subtask name, which becomes a field when its pencil is clicked. */
export function Title({
  title,
  done,
  className,
  onChange,
}: {
  title: string;
  done: boolean;
  className: string;
  onChange: (title: string) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);

  if (draft === null) {
    return (
      <span className="group/title flex min-w-0 flex-1 items-center gap-1">
        <span
          title={title}
          className={`min-w-0 truncate ${className} ${done ? "text-neutral line-through" : ""}`}
          onDoubleClick={() => setDraft(title)}
        >
          {title}
        </span>
        <button
          type="button"
          className="btn btn-ghost btn-xs px-1 opacity-0 transition-opacity duration-150 focus:opacity-100 group-hover/title:opacity-100"
          aria-label={`Rename ${title}`}
          onClick={() => setDraft(title)}
        >
          <Pencil size={12} aria-hidden="true" />
        </button>
      </span>
    );
  }

  const commit = () => {
    const name = draft.trim();
    if (name && name !== title) onChange(name);
    setDraft(null);
  };
  return (
    <input
      autoFocus
      className={`input input-xs input-bordered min-w-0 flex-1 ${className}`}
      aria-label={`New name for ${title}`}
      maxLength={200}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onFocus={(e) => e.currentTarget.select()}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") setDraft(null);
      }}
    />
  );
}

function SubtaskForm({
  task,
  onAdd,
}: {
  task: Task;
  onAdd: (title: string, minutes: number) => void;
}) {
  const [title, setTitle] = useState("");
  const [time, setTime] = useState("");
  const [error, setError] = useState<string | null>(null);

  const add = () => {
    const name = title.trim();
    if (!name) return;
    const minutes = time.trim() === "" ? 0 : parseDuration(time);
    if (minutes === null) {
      setError(TIME_HINT);
      return;
    }
    setError(null);
    onAdd(name, minutes);
    setTitle("");
    setTime("");
  };

  return (
    <form
      className="flex flex-wrap gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        add();
      }}
    >
      <input
        className="input input-sm input-bordered min-w-0 flex-1"
        placeholder="Add a subtask"
        aria-label={`Add a subtask to ${task.title}`}
        value={title}
        onChange={(e) => setTitle(e.target.value)}
      />
      <input
        className="input input-sm input-bordered w-24"
        placeholder="Time"
        aria-label={`Time for the new subtask of ${task.title}`}
        value={time}
        onChange={(e) => setTime(e.target.value)}
      />
      <button className="btn btn-ghost btn-sm" type="submit">
        Add
      </button>
      {error && (
        <p role="alert" className="w-full text-xs text-error">
          {error}
        </p>
      )}
    </form>
  );
}

function TaskRow({
  task,
  startOpen,
  spentSec,
  canFocus,
  onChange,
  onRemove,
  onFocus,
}: {
  task: Task;
  startOpen: boolean;
  spentSec: number;
  canFocus: boolean;
  onChange: (next: Task) => void;
  onRemove: () => void;
  onFocus: () => void;
}) {
  const [open, setOpen] = useState(startOpen);
  const [reward, setReward] = useState<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  const steps = task.subtasks.length;
  const stepsDone = task.subtasks.filter((s) => s.done).length;
  const over = subtasksOverBy(task);

  const celebrate = (xp: number) => {
    setReward(xp);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setReward(null), 1500);
  };
  const tick = () => {
    if (!task.done) celebrate(PRIORITY_INFO[task.priority].xp);
    onChange(toggleTask(task));
  };
  const tickStep = (id: number, wasDone: boolean) => {
    const next = toggleSubtask(task, id);
    if (!wasDone) {
      celebrate(SUBTASK_XP + (next.done ? PRIORITY_INFO[task.priority].xp : 0));
    }
    onChange(next);
  };

  return (
    <li className="group border-t border-base-300 py-2.5 first:border-t-0">
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          className="checkbox-primary checkbox checkbox-sm rounded-full"
          aria-label={`Finish ${task.title}`}
          checked={task.done}
          onChange={tick}
        />
        <Title
          title={task.title}
          done={task.done}
          className="text-[15px]"
          onChange={(title) => onChange({ ...task, title })}
        />
        {task.focus_area && (
          <span className="badge badge-ghost badge-sm whitespace-nowrap text-neutral">
            {task.focus_area}
          </span>
        )}
        {reward !== null && (
          <span className="animate-xp-pop whitespace-nowrap text-xs font-semibold text-accent">
            +{reward} XP
          </span>
        )}
        <PriorityChip
          label={`Priority of ${task.title}`}
          value={task.priority}
          onChange={(priority) => onChange({ ...task, priority })}
        />
        <TimePill
          label={`Time for ${task.title}`}
          minutes={task.allocated_min}
          spentSec={spentSec}
          onChange={(allocated_min) => onChange({ ...task, allocated_min })}
        />
        <button
          className="btn btn-ghost btn-xs w-24 justify-end gap-0.5 whitespace-nowrap font-normal text-neutral"
          aria-label={`${open ? "Hide" : "Show"} subtasks of ${task.title}`}
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          {steps > 0 ? `${stepsDone}/${steps}` : "Subtasks"}
          <ChevronRight
            size={14}
            aria-hidden="true"
            className={`transition-transform duration-150 ${open ? "rotate-90" : ""}`}
          />
        </button>
        <span className="flex opacity-0 transition-opacity duration-150 focus-within:opacity-100 group-hover:opacity-100">
          <button
            className="btn btn-ghost btn-xs px-1"
            aria-label={`Focus on ${task.title}`}
            disabled={!canFocus || task.done}
            onClick={onFocus}
          >
            <Play size={14} aria-hidden="true" />
          </button>
          <button
            className="btn btn-ghost btn-xs px-1"
            aria-label={`Remove ${task.title}`}
            onClick={onRemove}
          >
            <Trash2 size={14} aria-hidden="true" />
          </button>
        </span>
      </div>

      {open && (
        <div className="ml-7 mt-2 flex animate-fade-in flex-col gap-1.5">
          {task.subtasks.map((subtask) => (
            <div key={subtask.id} className="flex items-center gap-2">
              <input
                type="checkbox"
                className="checkbox-primary checkbox checkbox-xs rounded-full"
                aria-label={`Finish ${subtask.title}`}
                checked={subtask.done}
                onChange={() => tickStep(subtask.id, subtask.done)}
              />
              <Title
                title={subtask.title}
                done={subtask.done}
                className="text-sm"
                onChange={(title) =>
                  onChange({
                    ...task,
                    subtasks: task.subtasks.map((s) =>
                      s.id === subtask.id ? { ...s, title } : s,
                    ),
                  })
                }
              />
              <TimePill
                label={`Time for ${subtask.title}`}
                minutes={subtask.allocated_min}
                onChange={(allocated_min) =>
                  onChange({
                    ...task,
                    subtasks: task.subtasks.map((s) =>
                      s.id === subtask.id ? { ...s, allocated_min } : s,
                    ),
                  })
                }
              />
              <button
                className="btn btn-ghost btn-xs px-1"
                aria-label={`Remove ${subtask.title}`}
                onClick={() => onChange(removeSubtask(task, subtask.id))}
              >
                <X size={14} aria-hidden="true" />
              </button>
            </div>
          ))}
          {over > 0 && (
            <p className="text-xs text-error">
              Subtasks add up to {formatDuration(over * 60)} more than this task
              has.
            </p>
          )}
          <SubtaskForm
            task={task}
            onAdd={(title, minutes) =>
              onChange(addSubtask(task, title, minutes))
            }
          />
        </div>
      )}
    </li>
  );
}

function AddTaskForm({
  onAdd,
}: {
  onAdd: (title: string, priority: Priority, minutes: number) => Promise<void>;
}) {
  const [title, setTitle] = useState("");
  const [priority, setPriority] = useState<Priority>("medium");
  const [time, setTime] = useState("");
  const [error, setError] = useState<string | null>(null);

  const add = async () => {
    const name = title.trim();
    if (!name) return;
    const minutes = parseDuration(time);
    if (!minutes) {
      setError(`How long will this take? ${TIME_HINT}`);
      return;
    }
    setError(null);
    await onAdd(name, priority, minutes);
    setTitle("");
    setTime("");
    setPriority("medium");
  };

  return (
    <form
      className="flex flex-col gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <input
          className="input input-bordered min-w-52 flex-1"
          placeholder="Add a task"
          aria-label="New task"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <div
          role="group"
          aria-label="Priority"
          className="flex rounded-btn border border-base-300 p-1"
        >
          {PRIORITIES.map((value) => (
            <button
              key={value}
              type="button"
              aria-pressed={priority === value}
              className={`btn btn-sm border-0 font-medium ${
                priority === value
                  ? `bg-base-300 hover:bg-base-300 ${PRIORITY_INFO[value].tone}`
                  : "btn-ghost text-neutral"
              }`}
              onClick={() => setPriority(value)}
            >
              {PRIORITY_INFO[value].label}
            </button>
          ))}
        </div>
        <input
          className="input input-bordered w-32"
          placeholder="Total time"
          aria-label="Total time"
          value={time}
          onChange={(e) => setTime(e.target.value)}
        />
        <button className="btn btn-primary" type="submit">
          <Plus size={18} aria-hidden="true" />
          Add
        </button>
      </div>
      {error && (
        <p role="alert" className="text-sm text-error">
          {error}
        </p>
      )}
    </form>
  );
}

export default function Tasks() {
  const state = useStore((s) => s.snapshot?.state);
  const [error, setError] = useState<string | null>(null);
  const [justAdded, setJustAdded] = useState<number | null>(null);
  if (!state) return null;

  const { tasks } = state;
  const open = tasks.filter((task) => !task.done).length;
  const save = async (next: Task[]) =>
    setError(await applyCall(api.updateTasks(next)));

  return (
    <Card
      title="Today's tasks"
      action={
        <span className="text-sm text-neutral">
          {open === 0 && tasks.length > 0 ? "All done" : `${open} left`}
        </span>
      }
    >
      {tasks.length < MAX_TASKS ? (
        <AddTaskForm
          onAdd={async (title, priority, minutes) => {
            const next = addTask(
              tasks,
              title,
              priority,
              minutes,
              state.next_id,
            );
            // Open the new task, so its subtasks can be added next.
            setJustAdded(next[next.length - 1].id);
            await save(next);
          }}
        />
      ) : (
        <p role="status" className="text-sm text-neutral">
          That's {MAX_TASKS} tasks, the most for a day. Quick jobs go in small
          work below.
        </p>
      )}
      <ErrorNote message={error} />

      {tasks.length === 0 ? (
        <p className="py-8 text-center text-sm text-neutral">
          Nothing here yet. Add your first task above.
        </p>
      ) : (
        <ul className="flex flex-col">
          {sortTasks(tasks).map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              startOpen={task.id === justAdded}
              spentSec={state.today.focus_sec[task.title] ?? 0}
              canFocus={state.pomodoro.phase === "idle"}
              onChange={(next) =>
                void save(tasks.map((t) => (t.id === next.id ? next : t)))
              }
              onRemove={() => void save(tasks.filter((t) => t.id !== task.id))}
              onFocus={() =>
                void applyCall(api.startPomodoro(task.id)).then(setError)
              }
            />
          ))}
        </ul>
      )}
      <p className="text-xs text-neutral">
        Up to {MAX_TASKS} tasks a day. At 6 AM the list starts fresh, and
        today's tasks stay in your history.
      </p>
    </Card>
  );
}
