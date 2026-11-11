import { X } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { nextFreeId } from "../lib";
import { applyCall, useStore } from "../store";
import type { SmallItem } from "../types/SmallItem";
import { Card, ErrorNote } from "../ui";
import { Title } from "./Tasks";

function carriedLabel(date: string): string {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(year, month - 1, day).toLocaleDateString(undefined, {
    weekday: "short",
  });
}

/** Quick 5 to 10 minute jobs. Not tasks: no limit, no XP, and never a plan on their own. */
export default function SmallWork() {
  const state = useStore((s) => s.snapshot?.state);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const items = state.small_items;
  const left = items.filter((item) => !item.done).length;
  const save = async (next: SmallItem[]) =>
    setError(await applyCall(api.updateSmallItems(next)));
  const change = (id: number, update: Partial<SmallItem>) =>
    void save(items.map((i) => (i.id === id ? { ...i, ...update } : i)));

  const add = () => {
    const name = title.trim();
    if (!name) return;
    setTitle("");
    void save([
      ...items,
      {
        id: nextFreeId(items, state.next_id),
        date: state.today.date,
        title: name,
        done: false,
        carried_from_date: null,
      },
    ]);
  };

  return (
    <Card
      title="Small work"
      action={
        items.length > 0 && (
          <span className="text-sm text-neutral">
            {left === 0 ? "All done" : `${left} left`}
          </span>
        )
      }
    >
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <input
          className="input input-sm input-bordered min-w-0 flex-1"
          placeholder="Call the bank, reply to Sara"
          aria-label="New small work"
          maxLength={200}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <button className="btn btn-ghost btn-sm" type="submit">
          Add
        </button>
      </form>
      <ErrorNote message={error} />
      {items.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {items.map((item) => (
            <li key={item.id} className="flex items-center gap-2">
              <input
                type="checkbox"
                className="checkbox-primary checkbox checkbox-xs rounded-full"
                aria-label={`Finish ${item.title}`}
                checked={item.done}
                onChange={() => change(item.id, { done: !item.done })}
              />
              <Title
                title={item.title}
                done={item.done}
                className="text-sm"
                onChange={(next) => change(item.id, { title: next })}
              />
              {item.carried_from_date && (
                <span className="whitespace-nowrap text-xs text-neutral">
                  from {carriedLabel(item.carried_from_date)}
                </span>
              )}
              <button
                className="btn btn-ghost btn-xs px-1"
                aria-label={`Remove ${item.title}`}
                onClick={() => void save(items.filter((i) => i.id !== item.id))}
              >
                <X size={14} aria-hidden="true" />
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="text-xs text-neutral">
        Quick jobs of 5 to 10 minutes. They don't count as tasks, and unticked
        ones move to the next day.
      </p>
    </Card>
  );
}
