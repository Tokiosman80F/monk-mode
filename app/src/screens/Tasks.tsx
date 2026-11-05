import { ChevronRight, Play, Plus, Trash2, X } from "lucide-react";
import { useRef, useState } from "react";
import { api } from "../api";
import {
  PRIORITIES,
  PRIORITY_INFO,
  SUBTASK_XP,
  addSubtask,
  addTask,
  removeSubtask,
  sortTasks,
  toggleSubtask,
  toggleTask,
} from "../lib";
import { applyCall, useStore } from "../store";
import type { Priority } from "../types/Priority";
import type { Task } from "../types/Task";
import { Card, ErrorNote } from "../ui";

function PrioritySelect({
  value,
  label,
  onChange,
}: {
  value: Priority;
  label: string;
  onChange: (priority: Priority) => void;
}) {
  return (
    <select
      aria-label={label}
      className={`select select-ghost select-sm ${PRIORITY_INFO[value].tone}`}
      value={value}
      onChange={(e) => onChange(e.target.value as Priority)}
    >
      {PRIORITIES.map((priority) => (
        <option key={priority} value={priority}>
          {PRIORITY_INFO[priority].label}
        </option>
      ))}
    </select>
  );
}

function TaskRow({
  task,
  canFocus,
  onChange,
  onRemove,
  onFocus,
}: {
  task: Task;
  canFocus: boolean;
  onChange: (next: Task) => void;
  onRemove: () => void;
  onFocus: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState("");
  const [reward, setReward] = useState<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  const steps = task.subtasks.length;
  const stepsDone = task.subtasks.filter((s) => s.done).length;

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
  const add = () => {
    const title = input.trim();
    if (!title) return;
    onChange(addSubtask(task, title));
    setInput("");
  };

  return (
    <li className="border-t border-base-300 py-2 first:border-t-0">
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          className="checkbox-primary checkbox checkbox-sm rounded-full"
          aria-label={`Finish ${task.title}`}
          checked={task.done}
          onChange={tick}
        />
        <button
          className="btn btn-ghost btn-xs px-1"
          aria-label={`${open ? "Hide" : "Show"} subtasks of ${task.title}`}
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <ChevronRight
            size={16}
            aria-hidden="true"
            className={`transition-transform duration-150 ${open ? "rotate-90" : ""}`}
          />
        </button>
        <span
          className={`min-w-0 flex-1 truncate ${task.done ? "text-neutral line-through" : ""}`}
        >
          {task.title}
        </span>
        {reward !== null && (
          <span className="animate-xp-pop font-display text-sm font-bold text-primary">
            +{reward} XP
          </span>
        )}
        {steps > 0 && (
          <span className="text-xs tabular-nums text-neutral">
            {stepsDone}/{steps}
          </span>
        )}
        <PrioritySelect
          label={`Priority of ${task.title}`}
          value={task.priority}
          onChange={(priority) => onChange({ ...task, priority })}
        />
        <button
          className="btn btn-ghost btn-xs"
          aria-label={`Focus on ${task.title}`}
          disabled={!canFocus || task.done}
          onClick={onFocus}
        >
          <Play size={15} aria-hidden="true" />
        </button>
        <button
          className="btn btn-ghost btn-xs"
          aria-label={`Remove ${task.title}`}
          onClick={onRemove}
        >
          <Trash2 size={15} aria-hidden="true" />
        </button>
      </div>

      {open && (
        <div className="ml-[52px] mt-1 flex animate-fade-in flex-col gap-1">
          {task.subtasks.map((subtask) => (
            <div key={subtask.id} className="flex items-center gap-2">
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
              <button
                className="btn btn-ghost btn-xs"
                aria-label={`Remove ${subtask.title}`}
                onClick={() => onChange(removeSubtask(task, subtask.id))}
              >
                <X size={14} aria-hidden="true" />
              </button>
            </div>
          ))}
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              add();
            }}
          >
            <input
              className="input input-sm input-bordered min-w-0 flex-1"
              placeholder="Add a subtask"
              aria-label={`Add a subtask to ${task.title}`}
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <button className="btn btn-ghost btn-sm" type="submit">
              Add
            </button>
          </form>
        </div>
      )}
    </li>
  );
}

export default function Tasks() {
  const state = useStore((s) => s.snapshot?.state);
  const [title, setTitle] = useState("");
  const [priority, setPriority] = useState<Priority>("medium");
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const { tasks } = state;
  const open = tasks.filter((task) => !task.done).length;
  const save = async (next: Task[]) =>
    setError(await applyCall(api.updateTasks(next)));
  const add = async () => {
    const name = title.trim();
    if (!name) return;
    await save(addTask(tasks, name, priority));
    setTitle("");
  };

  return (
    <Card
      title="Today's tasks"
      action={
        <span className="text-sm text-neutral">
          {open === 0 && tasks.length > 0 ? "All done" : `${open} left`}
        </span>
      }
    >
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <input
          className="input input-bordered min-w-0 flex-1"
          placeholder="What needs doing today?"
          aria-label="New task"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <PrioritySelect
          label="New task priority"
          value={priority}
          onChange={setPriority}
        />
        <button className="btn btn-primary" type="submit">
          <Plus size={18} aria-hidden="true" />
          Add
        </button>
      </form>
      <ErrorNote message={error} />

      {tasks.length === 0 ? (
        <p className="py-6 text-center text-sm text-neutral">
          Add your first task. High priority earns 50 XP, medium 30 and low 15.
        </p>
      ) : (
        <ul className="flex flex-col">
          {sortTasks(tasks).map((task) => (
            <TaskRow
              key={task.id}
              task={task}
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
        Unfinished tasks carry over to tomorrow.
      </p>
    </Card>
  );
}
