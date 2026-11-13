import { Plus, X } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { applyCall, useStore } from "../store";
import type { Profile } from "../types/Profile";
import { ErrorNote, Logo } from "../ui";

/** Hours as typed, like `2.5`, to whole minutes. Null when it isn't a number of hours in a day. */
function hoursToMin(text: string): number | null {
  const hours = Number(text);
  if (text.trim() === "" || !Number.isFinite(hours) || hours < 0 || hours > 24)
    return null;
  return Math.round(hours * 60);
}

/**
 * The planner profile: name and focus areas, plus the daily study hours when
 * `studyMin` is given. Used by the one-time setup and by Settings.
 */
export function ProfileForm({
  profile,
  studyMin,
  submitLabel,
  onSave,
}: {
  profile: Profile | null;
  studyMin?: number;
  submitLabel: string;
  onSave: (profile: Profile, studyMin: number | null) => Promise<string | null>;
}) {
  const [name, setName] = useState(profile?.name ?? "");
  const [studyHours, setStudyHours] = useState(
    studyMin === undefined ? "" : String(studyMin / 60),
  );
  const [areas, setAreas] = useState<string[]>(
    profile?.focus_areas.length ? profile.focus_areas.map((a) => a.name) : [""],
  );
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const setArea = (index: number, value: string) => {
    setSaved(false);
    setAreas(areas.map((a, i) => (i === index ? value : a)));
  };

  const save = async () => {
    setSaved(false);
    if (!name.trim()) return setError("Enter your name.");
    let study: number | null = null;
    if (studyMin !== undefined) {
      study = hoursToMin(studyHours);
      if (study === null)
        return setError("Daily study hours must be between 0 and 24.");
    }
    const named = areas.map((a) => a.trim()).filter(Boolean);
    if (named.length === 0) return setError("Add at least one focus area.");
    const failure = await onSave(
      { name: name.trim(), focus_areas: named.map((n) => ({ name: n })) },
      study,
    );
    setError(failure);
    setSaved(failure === null);
  };

  return (
    <form
      className="flex flex-col gap-6"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <label className="flex flex-col gap-1.5">
        <span className="text-sm font-medium">Your name</span>
        <input
          className="input input-bordered"
          autoFocus={!profile}
          maxLength={60}
          value={name}
          onChange={(e) => {
            setSaved(false);
            setName(e.target.value);
          }}
        />
      </label>

      {studyMin !== undefined && (
        <div className="flex flex-col gap-1.5">
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">Daily study hours</span>
            <input
              className="input input-bordered w-32"
              inputMode="decimal"
              placeholder="8"
              value={studyHours}
              onChange={(e) => setStudyHours(e.target.value)}
            />
          </label>
          <span className="text-sm text-neutral">
            Your daily focus target. Hit it to keep your streak.
          </span>
        </div>
      )}

      <fieldset className="flex flex-col gap-2">
        <legend className="mb-1.5 text-sm font-medium">Focus areas</legend>
        <p className="-mt-1 text-sm text-neutral">
          What you work on, like React or Marketing. Each task gets one.
        </p>
        {areas.map((area, i) => (
          <div key={i} className="flex items-center gap-2">
            <input
              className="input input-sm input-bordered min-w-0 flex-1"
              aria-label={`Focus area ${i + 1}`}
              placeholder={i === 0 ? "React" : "Another area"}
              maxLength={60}
              value={area}
              onChange={(e) => setArea(i, e.target.value)}
            />
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
            onClick={() => setAreas([...areas, ""])}
          >
            <Plus size={16} aria-hidden="true" />
            Add an area
          </button>
        )}
      </fieldset>

      <ErrorNote message={error} />
      <div className="flex items-center gap-4">
        <button className="btn btn-primary" type="submit">
          {submitLabel}
        </button>
        {saved && profile && (
          <span role="status" className="text-sm text-neutral">
            Saved.
          </span>
        )}
      </div>
    </form>
  );
}

/** The planner's one-time setup, shown before the first morning prompt. */
export default function PlannerSetup() {
  const settings = useStore((s) => s.snapshot?.state.settings);
  if (!settings) return null;

  return (
    <main className="min-h-screen overflow-y-auto bg-base-100 text-base-content">
      <div className="mx-auto flex max-w-xl animate-fade-in flex-col gap-6 px-6 py-12">
        <header className="flex flex-col gap-3">
          <Logo className="w-10" />
          <h1 className="font-display text-3xl font-light tracking-tight">
            Set up your planner
          </h1>
          <p className="text-neutral">
            Last step. You can change all of this later in Settings.
          </p>
        </header>
        <ProfileForm
          profile={null}
          studyMin={settings.daily_focus_target_min}
          submitLabel="Continue to today's plan"
          onSave={(profile, studyMin) =>
            applyCall(
              api.updateSettings({
                ...settings,
                daily_focus_target_min:
                  studyMin ?? settings.daily_focus_target_min,
                profile,
              }),
            )
          }
        />
      </div>
    </main>
  );
}
