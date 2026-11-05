import { useEffect, useState } from "react";
import { api } from "../api";
import { applyCall, useStore } from "../store";
import type { Settings } from "../types/Settings";
import { Card, ErrorNote, PageTitle } from "../ui";

function NumberField({
  label,
  value,
  min = 0,
  max,
  step = 1,
  hint,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  hint?: string;
  onChange: (value: number) => void;
}) {
  return (
    <div className="form-control">
      <label className="form-control">
        <span className="label-text mb-1">{label}</span>
        <input
          type="number"
          className="input input-bordered w-32"
          value={value}
          min={min}
          max={max}
          step={step}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      </label>
      {hint && <span className="mt-1 text-sm text-neutral">{hint}</span>}
    </div>
  );
}

export default function FocusSettings() {
  const saved = useStore((s) => s.snapshot?.state.settings);
  const [draft, setDraft] = useState<Settings | undefined>(saved);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  // First load only: a push every second must not overwrite what's being typed.
  useEffect(() => {
    if (saved && !draft) setDraft(saved);
  }, [saved, draft]);

  if (!saved || !draft) return <PageTitle>Settings</PageTitle>;

  const update = (change: Partial<Settings>) => {
    setDone(false);
    setDraft({ ...draft, ...change });
  };

  const save = async () => {
    // Only these fields are edited here. The rest can change elsewhere while this is open.
    const next: Settings = {
      ...saved,
      pomodoro_focus_min: draft.pomodoro_focus_min,
      pomodoro_break_min: draft.pomodoro_break_min,
      daily_focus_target_min: draft.daily_focus_target_min,
    };
    const failure = await applyCall(api.updateSettings(next));
    setError(failure);
    setDone(!failure);
  };

  return (
    <>
      <PageTitle>Settings</PageTitle>

      <Card title="Focus sessions">
        <div className="flex gap-6">
          <NumberField
            label="Focus minutes"
            value={draft.pomodoro_focus_min}
            min={1}
            onChange={(value) => update({ pomodoro_focus_min: value })}
          />
          <NumberField
            label="Break minutes"
            value={draft.pomodoro_break_min}
            onChange={(value) => update({ pomodoro_break_min: value })}
          />
        </div>
      </Card>

      <Card title="Daily goal">
        <NumberField
          label="Daily focus target (hours)"
          value={draft.daily_focus_target_min / 60}
          max={24}
          step={0.5}
          hint="Hit this to keep your streak."
          onChange={(hours) =>
            update({ daily_focus_target_min: Math.round(hours * 60) })
          }
        />
      </Card>

      <ErrorNote message={error} />
      <div className="flex items-center gap-4">
        <button className="btn btn-primary" onClick={() => void save()}>
          Save
        </button>
        {done && <span className="text-sm text-neutral">Saved.</span>}
      </div>
    </>
  );
}
