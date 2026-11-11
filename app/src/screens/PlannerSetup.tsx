import { Plus, X } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { applyCall, useStore } from "../store";
import type { FocusArea } from "../types/FocusArea";
import type { WeekStart } from "../types/WeekStart";
import { ErrorNote, Logo } from "../ui";

const WEEK_STARTS: [WeekStart, string][] = [
  ["saturday", "Saturday"],
  ["sunday", "Sunday"],
  ["monday", "Monday"],
];

const DEFAULT_AREAS: FocusArea[] = [
  { name: "React", target_min: 180 },
  { name: "Products", target_min: 360 },
  { name: "LinkedIn", target_min: 120 },
];

/** Hours as typed, like `2.5`, to whole minutes. Null when it isn't a number of hours in a day. */
function hoursToMin(text: string): number | null {
  const hours = Number(text);
  if (text.trim() === "" || !Number.isFinite(hours) || hours < 0 || hours > 24)
    return null;
  return Math.round(hours * 60);
}

interface AreaDraft {
  name: string;
  hours: string;
}

/** The planner's one-time setup, shown before the first morning prompt. */
export default function PlannerSetup() {
  const settings = useStore((s) => s.snapshot?.state.settings);
  const [name, setName] = useState("");
  const [studyHours, setStudyHours] = useState(
    String((settings?.daily_focus_target_min ?? 660) / 60),
  );
  const [areas, setAreas] = useState<AreaDraft[]>(
    DEFAULT_AREAS.map((a) => ({
      name: a.name,
      hours: String(a.target_min / 60),
    })),
  );
  const [weekStart, setWeekStart] = useState<WeekStart>("saturday");
  const [error, setError] = useState<string | null>(null);
  if (!settings) return null;

  const setArea = (index: number, change: Partial<AreaDraft>) =>
    setAreas(areas.map((a, i) => (i === index ? { ...a, ...change } : a)));

  const save = async () => {
    const study = hoursToMin(studyHours);
    if (!name.trim()) return setError("Enter your name.");
    if (study === null)
      return setError("Daily study hours must be between 0 and 24.");
    const named = areas.filter((a) => a.name.trim());
    if (named.length === 0) return setError("Add at least one focus area.");
    const focusAreas: FocusArea[] = [];
    for (const area of named) {
      const target = hoursToMin(area.hours);
      if (target === null)
        return setError(`Hours for ${area.name} must be between 0 and 24.`);
      focusAreas.push({ name: area.name.trim(), target_min: target });
    }
    setError(
      await applyCall(
        api.updateSettings({
          ...settings,
          daily_focus_target_min: study,
          profile: {
            name: name.trim(),
            focus_areas: focusAreas,
            week_start: weekStart,
          },
        }),
      ),
    );
  };

  return (
    <main className="min-h-screen overflow-y-auto bg-base-100 text-base-content">
      <form
        className="mx-auto flex max-w-xl animate-fade-in flex-col gap-6 px-6 py-12"
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <header className="flex flex-col gap-3">
          <Logo className="w-10" />
          <h1 className="font-display text-3xl font-light tracking-tight">
            Set up your planner
          </h1>
          <p className="text-neutral">
            This happens once, before your first morning plan.
          </p>
        </header>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium">Your name</span>
          <input
            className="input input-bordered"
            autoFocus
            maxLength={60}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium">Daily study hours</span>
          <input
            className="input input-bordered w-32"
            inputMode="decimal"
            value={studyHours}
            onChange={(e) => setStudyHours(e.target.value)}
          />
        </label>

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1.5 text-sm font-medium">
            Focus areas and hours a day
          </legend>
          {areas.map((area, i) => (
            <div key={i} className="flex items-center gap-2">
              <input
                className="input input-sm input-bordered min-w-0 flex-1"
                aria-label={`Focus area ${i + 1}`}
                maxLength={60}
                value={area.name}
                onChange={(e) => setArea(i, { name: e.target.value })}
              />
              <input
                className="input input-sm input-bordered w-20"
                aria-label={`Hours a day for focus area ${i + 1}`}
                inputMode="decimal"
                value={area.hours}
                onChange={(e) => setArea(i, { hours: e.target.value })}
              />
              <span className="text-sm text-neutral">h</span>
              <button
                type="button"
                className="btn btn-ghost btn-xs px-1"
                aria-label={`Remove focus area ${i + 1}`}
                onClick={() => setAreas(areas.filter((_, j) => j !== i))}
              >
                <X size={14} aria-hidden="true" />
              </button>
            </div>
          ))}
          {areas.length < 6 && (
            <button
              type="button"
              className="btn btn-ghost btn-sm self-start"
              onClick={() => setAreas([...areas, { name: "", hours: "1" }])}
            >
              <Plus size={16} aria-hidden="true" />
              Add an area
            </button>
          )}
        </fieldset>

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1.5 text-sm font-medium">
            Your week starts on
          </legend>
          <div className="flex gap-2">
            {WEEK_STARTS.map(([value, label]) => (
              <button
                key={value}
                type="button"
                aria-pressed={weekStart === value}
                className={`btn btn-sm ${weekStart === value ? "btn-secondary" : "btn-ghost border-base-300"}`}
                onClick={() => setWeekStart(value)}
              >
                {label}
              </button>
            ))}
          </div>
        </fieldset>

        <ErrorNote message={error} />
        <button className="btn btn-primary self-start" type="submit">
          Continue to today&apos;s plan
        </button>
      </form>
    </main>
  );
}
