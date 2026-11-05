import { ChevronRight, Play, Plus, Trash2, X } from "lucide-react";
import { useRef, useState, type DragEvent } from "react";
import { api } from "../api";
import {
  PRIORITY_INFO,
  QUADRANTS,
  QUADRANT_INFO,
  SUBTASK_XP,
  addSubtask,
  addTask,
  formatDuration,
  nextPriority,
  parseDuration,
  plannedMin,
  quadrantOf,
  removeSubtask,
  sortTasks,
  subtasksOverBy,
  toggleSubtask,
  toggleTask,
} from "../lib";
import { applyCall, useStore } from "../store";
import type { Priority } from "../types/Priority";
import type { Quadrant } from "../types/Quadrant";
import type { Task } from "../types/Task";
import { ErrorNote } from "../ui";

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
      className={`btn btn-ghost btn-xs font-medium ${info.tone}`}
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
        className={`btn btn-ghost btn-xs whitespace-nowrap font-normal tabular-nums ${minutes === 0 ? "text-neutral" : ""}`}
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
      className="input input-xs input-bordered w-20"
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
        className="input input-sm input-bordered w-20"
        placeholder="30m"
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
  spentSec,
  canFocus,
  onChange,
  onRemove,
  onFocus,
}: {
  task: Task;
  spentSec: number;
  canFocus: boolean;
  onChange: (next: Task) => void;
  onRemove: () => void;
  onFocus: () => void;
}) {
  const [open, setOpen] = useState(false);
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
    <li
      draggable
      onDragStart={(e) => e.dataTransfer.setData("text/plain", String(task.id))}
      className="group rounded-btn bg-base-100/60 px-2 py-1.5"
    >
      <div className="flex items-center gap-1.5">
        <input
          type="checkbox"
          className="checkbox-primary checkbox checkbox-sm rounded-full"
          aria-label={`Finish ${task.title}`}
          checked={task.done}
          onChange={tick}
        />
        <button
          className="btn btn-ghost btn-xs px-0.5"
          aria-label={`${open ? "Hide" : "Show"} subtasks of ${task.title}`}
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <ChevronRight
            size={15}
            aria-hidden="true"
            className={`transition-transform duration-150 ${open ? "rotate-90" : ""}`}
          />
        </button>
        <span
          title={task.title}
          className={`min-w-0 flex-1 truncate ${task.done ? "text-neutral line-through" : ""}`}
        >
          {task.title}
        </span>
        {reward !== null && (
          <span className="animate-xp-pop whitespace-nowrap text-xs font-semibold text-accent">
            +{reward} XP
          </span>
        )}
        {steps > 0 && (
          <span className="text-xs tabular-nums text-neutral">
            {stepsDone}/{steps}
          </span>
        )}
        <TimePill
          label={`Time for ${task.title}`}
          minutes={task.allocated_min}
          spentSec={spentSec}
          onChange={(allocated_min) => onChange({ ...task, allocated_min })}
        />
        <PriorityChip
          label={`Priority of ${task.title}`}
          value={task.priority}
          onChange={(priority) => onChange({ ...task, priority })}
        />
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

      {task.allocated_min > 0 && !task.done && (
        <progress
          className="progress progress-secondary mt-1 block h-0.5 w-full"
          aria-label={`Time spent on ${task.title}`}
          value={spentSec}
          max={task.allocated_min * 60}
        />
      )}

      {open && (
        <div className="ml-7 mt-2 flex animate-fade-in flex-col gap-1.5 pb-1">
          {task.subtasks.map((subtask) => (
            <div key={subtask.id} className="flex items-center gap-1.5">
              <input
                type="checkbox"
                className="checkbox-primary checkbox checkbox-xs rounded-full"
                aria-label={`Finish ${subtask.title}`}
                checked={subtask.done}
                onChange={() => tickStep(subtask.id, subtask.done)}
              />
              <span
                className={`min-w-0 flex-1 truncate text-sm ${subtask.done ? "text-neutral line-through" : ""}`}
              >
                {subtask.title}
              </span>
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
          <div className="flex flex-wrap items-center gap-1 text-xs text-neutral">
            <span>Move to</span>
            {QUADRANTS.filter((q) => q !== task.quadrant).map((quadrant) => (
              <button
                key={quadrant}
                className={`btn btn-ghost btn-xs font-normal ${QUADRANT_INFO[quadrant].text}`}
                onClick={() => onChange({ ...task, quadrant })}
              >
                {QUADRANT_INFO[quadrant].name}
              </button>
            ))}
          </div>
        </div>
      )}
    </li>
  );
}

function Composer({
  onAdd,
}: {
  onAdd: (
    title: string,
    minutes: number,
    priority: Priority,
    quadrant: Quadrant,
  ) => Promise<void>;
}) {
  const [title, setTitle] = useState("");
  const [time, setTime] = useState("");
  const [urgent, setUrgent] = useState(false);
  const [important, setImportant] = useState(true);
  // Follows the quadrant until it's picked by hand.
  const [picked, setPicked] = useState<Priority | null>(null);
  const [error, setError] = useState<string | null>(null);
  const quadrant = quadrantOf(urgent, important);
  const info = QUADRANT_INFO[quadrant];
  const priority = picked ?? info.priority;

  const add = async () => {
    const name = title.trim();
    if (!name) return;
    const minutes = parseDuration(time);
    if (!minutes) {
      setError(`How long will this take? ${TIME_HINT}`);
      return;
    }
    setError(null);
    await onAdd(name, minutes, priority, quadrant);
    setTitle("");
    setTime("");
    setPicked(null);
  };

  const toggle = (
    on: boolean,
    text: string,
    change: (next: boolean) => void,
  ) => (
    <button
      type="button"
      aria-pressed={on}
      className={`btn btn-sm font-medium ${on ? "btn-primary" : "btn-ghost text-neutral"}`}
      onClick={() => change(!on)}
    >
      {text}
    </button>
  );

  return (
    <form
      className="flex flex-col gap-2 rounded-box border border-base-300 bg-base-200 p-3"
      onSubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <input
          className="input input-sm input-ghost min-w-48 flex-1 text-base focus:bg-transparent"
          placeholder="What needs doing today?"
          aria-label="New task"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <input
          className="input input-sm input-bordered w-24"
          placeholder="1h 30m"
          aria-label="Time for the new task"
          value={time}
          onChange={(e) => setTime(e.target.value)}
        />
        <button className="btn btn-primary btn-sm" type="submit">
          <Plus size={16} aria-hidden="true" />
          Add
        </button>
      </div>
      <div className="flex flex-wrap items-center gap-2 pl-1">
        <span className="text-xs text-neutral">Is it</span>
        {toggle(urgent, "Urgent", setUrgent)}
        {toggle(important, "Important", setImportant)}
        <span className={`text-sm font-medium ${info.text}`}>{info.name}</span>
        <span className="ml-auto flex items-center gap-1 text-xs text-neutral">
          Priority
          <PriorityChip
            label="New task priority"
            value={priority}
            onChange={setPicked}
          />
        </span>
      </div>
      {error && (
        <p role="alert" className="pl-1 text-sm text-error">
          {error}
        </p>
      )}
    </form>
  );
}

function QuadrantPanel({
  quadrant,
  tasks,
  children,
  onDropTask,
}: {
  quadrant: Quadrant;
  tasks: Task[];
  children: React.ReactNode;
  onDropTask: (id: number) => void;
}) {
  const info = QUADRANT_INFO[quadrant];
  const [over, setOver] = useState(false);
  const planned = plannedMin(tasks);
  const drop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    const id = Number(e.dataTransfer.getData("text/plain"));
    if (id) onDropTask(id);
  };

  return (
    <section
      aria-label={info.name}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={drop}
      className={`flex min-h-44 min-w-0 flex-col gap-2 rounded-box border-t-2 bg-base-200 p-3 transition-colors duration-150 ${info.border} ${over ? "bg-base-300" : ""}`}
    >
      <header className="flex items-baseline justify-between gap-3 px-1">
        <div className="min-w-0">
          <h2 className={`font-display text-lg font-medium ${info.text}`}>
            {info.name}
          </h2>
          <p className="truncate text-xs text-neutral">{info.advice}</p>
        </div>
        {planned > 0 && (
          <span className="whitespace-nowrap text-xs tabular-nums text-neutral">
            {formatDuration(planned * 60)}
          </span>
        )}
      </header>
      {children}
    </section>
  );
}

const AXIS = "text-xs text-neutral";

export default function Tasks() {
  const state = useStore((s) => s.snapshot?.state);
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const { tasks } = state;
  const save = async (next: Task[]) =>
    setError(await applyCall(api.updateTasks(next)));
  const update = (next: Task) =>
    void save(tasks.map((t) => (t.id === next.id ? next : t)));

  const panel = (quadrant: Quadrant) => {
    const own = tasks.filter((task) => task.quadrant === quadrant);
    return (
      <QuadrantPanel
        key={quadrant}
        quadrant={quadrant}
        tasks={own}
        onDropTask={(id) => {
          const task = tasks.find((t) => t.id === id);
          if (task && task.quadrant !== quadrant) update({ ...task, quadrant });
        }}
      >
        {own.length === 0 ? (
          <p className="flex flex-1 items-center justify-center p-4 text-center text-xs text-neutral">
            {QUADRANT_INFO[quadrant].empty}
          </p>
        ) : (
          <ul className="flex flex-col gap-1">
            {sortTasks(own).map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                spentSec={state.today.focus_sec[task.title] ?? 0}
                canFocus={state.pomodoro.phase === "idle"}
                onChange={update}
                onRemove={() =>
                  void save(tasks.filter((t) => t.id !== task.id))
                }
                onFocus={() =>
                  void applyCall(api.startPomodoro(task.id)).then(setError)
                }
              />
            ))}
          </ul>
        )}
      </QuadrantPanel>
    );
  };

  return (
    <section aria-label="Today's tasks" className="flex flex-col gap-3">
      <Composer
        onAdd={(title, minutes, priority, quadrant) =>
          save(addTask(tasks, title, priority, quadrant, minutes))
        }
      />
      <ErrorNote message={error} />

      <div className="grid gap-2 md:grid-cols-[auto_minmax(0,1fr)_minmax(0,1fr)]">
        <span className="hidden md:block" />
        <span className={`hidden text-center md:block ${AXIS}`}>Urgent</span>
        <span className={`hidden text-center md:block ${AXIS}`}>
          Not urgent
        </span>
        <span
          className={`hidden rotate-180 self-center [writing-mode:vertical-rl] md:block ${AXIS}`}
        >
          Important
        </span>
        {panel("do_first")}
        {panel("schedule")}
        <span
          className={`hidden rotate-180 self-center [writing-mode:vertical-rl] md:block ${AXIS}`}
        >
          Not important
        </span>
        {panel("delegate")}
        {panel("eliminate")}
      </div>
      <p className="text-xs text-neutral">
        Drag a task to another box to move it. Unfinished tasks carry over to
        tomorrow.
      </p>
    </section>
  );
}
