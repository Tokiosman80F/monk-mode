import { useState } from "react";
import { api } from "../api";
import { applyCall, useStore } from "../store";
import type { Settings } from "../types/Settings";
import { Card, ErrorNote, PageTitle } from "../ui";
import { ProfileForm } from "./PlannerSetup";

/** Saves when the field is left, like the limits, so typing "25" doesn't save "2" first. */
function NumberField({
  label,
  value,
  min = 0,
  max,
  step = 1,
  hint,
  onSave,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  hint?: string;
  onSave: (value: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  const commit = () => {
    const next = Number(draft);
    const ok =
      draft.trim() !== "" &&
      Number.isFinite(next) &&
      next >= min &&
      (max === undefined || next <= max);
    if (ok && next !== value) onSave(next);
    else if (!ok) setDraft(String(value));
  };
  return (
    <div className="form-control">
      <label className="form-control">
        <span className="label-text mb-1">{label}</span>
        <input
          type="number"
          className="input input-bordered w-32"
          value={draft}
          min={min}
          max={max}
          step={step}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
      </label>
      {hint && <span className="mt-1 text-sm text-neutral">{hint}</span>}
    </div>
  );
}

export default function FocusSettings() {
  const settings = useStore((s) => s.snapshot?.state.settings);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  if (!settings) return <PageTitle>Settings</PageTitle>;

  // Changes save on their own, from the latest settings, so nothing else is overwritten.
  const save = async (change: Partial<Settings>) => {
    const failure = await applyCall(
      api.updateSettings({ ...settings, ...change }),
    );
    setError(failure);
    setSaved(failure === null);
  };

  return (
    <>
      <PageTitle>Settings</PageTitle>
      <p className="-mt-2 text-sm text-neutral">
        Changes save when you leave a field.
        {saved && <span role="status"> Saved.</span>}
      </p>

      <Card title="Focus sessions">
        <div className="flex gap-6">
          <NumberField
            label="Focus minutes"
            value={settings.pomodoro_focus_min}
            min={1}
            max={180}
            onSave={(value) => void save({ pomodoro_focus_min: value })}
          />
          <NumberField
            label="Break minutes"
            value={settings.pomodoro_break_min}
            max={60}
            onSave={(value) => void save({ pomodoro_break_min: value })}
          />
        </div>
      </Card>

      <Card title="Daily goal">
        <NumberField
          label="Daily focus target (hours)"
          value={settings.daily_focus_target_min / 60}
          max={24}
          step={0.5}
          hint="Hit this to keep your streak."
          onSave={(hours) =>
            void save({ daily_focus_target_min: Math.round(hours * 60) })
          }
        />
      </Card>
      <ErrorNote message={error} />

      <Card title="Your planner">
        <ProfileForm
          profile={settings.profile}
          submitLabel="Save planner"
          onSave={(profile) =>
            applyCall(api.updateSettings({ ...settings, profile }))
          }
        />
      </Card>
    </>
  );
}
