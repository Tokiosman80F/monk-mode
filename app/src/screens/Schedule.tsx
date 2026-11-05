import { useEffect, useState } from "react";
import { api } from "../api";
import { isLocked, lockedUntil } from "../lib";
import { applyCall, useStore } from "../store";
import type { Settings } from "../types/Settings";
import { Card, ErrorNote, PageTitle } from "../ui";

const MAX_ALLOWANCE_MIN = 60;

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

export default function Schedule() {
  const snapshot = useStore((s) => s.snapshot);
  const saved = snapshot?.state.settings;
  const [draft, setDraft] = useState<Settings | undefined>(saved);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  // First load only: a push every second must not overwrite what's being typed.
  useEffect(() => {
    if (saved && !draft) setDraft(saved);
  }, [saved, draft]);

  if (!saved || !draft || !snapshot) return <PageTitle>Schedule</PageTitle>;

  const { lock } = snapshot.state;
  const locked = isLocked(lock);
  const maxAllowance = locked ? saved.daily_allowance_min : MAX_ALLOWANCE_MIN;
  const update = (change: Partial<Settings>) => {
    setDone(false);
    setDraft({ ...draft, ...change });
  };
  const updateBlock = (
    index: number,
    change: Partial<Settings["study_blocks"][number]>,
  ) =>
    update({
      study_blocks: draft.study_blocks.map((block, i) =>
        i === index ? { ...block, ...change } : block,
      ),
    });

  const save = async () => {
    // Lists can change elsewhere while this screen is open, and they aren't edited here.
    const next: Settings = {
      ...draft,
      allowance_sites: saved.allowance_sites,
      always_blocked: saved.always_blocked,
      shorts_blocker_enabled: saved.shorts_blocker_enabled,
    };
    const failure = await applyCall(api.updateSettings(next));
    setError(failure);
    setDone(!failure);
  };

  return (
    <>
      <PageTitle>Schedule</PageTitle>

      <Card title="Daily allowance">
        <NumberField
          label="Minutes per day"
          value={draft.daily_allowance_min}
          max={maxAllowance}
          hint={
            locked
              ? `${lockedUntil(lock)}. You can only lower this.`
              : "From 0 to 60."
          }
          onChange={(value) => update({ daily_allowance_min: value })}
        />
      </Card>

      <Card title="Pomodoro">
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

      <Card title="Study blocks">
        {draft.study_blocks.map((block, i) => (
          <div key={i} className="flex items-end gap-4">
            <label className="form-control flex-1">
              <span className="label-text mb-1">Name</span>
              <input
                className="input input-bordered w-full"
                value={block.name}
                onChange={(e) => updateBlock(i, { name: e.target.value })}
              />
            </label>
            <NumberField
              label="Hours"
              value={block.target_min / 60}
              step={0.5}
              onChange={(hours) =>
                updateBlock(i, { target_min: Math.round(hours * 60) })
              }
            />
          </div>
        ))}
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
