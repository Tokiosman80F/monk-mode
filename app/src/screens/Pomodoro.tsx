import { Play, Square } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { formatClock } from "../lib";
import { applyCall, useStore } from "../store";
import { Card, ErrorNote, Logo } from "../ui";

export default function Pomodoro() {
  const state = useStore((s) => s.snapshot?.state);
  const [choice, setChoice] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!state) return null;

  const blocks = state.settings.study_blocks;
  const block = blocks.some((b) => b.name === choice)
    ? choice
    : (blocks[0]?.name ?? "");
  const { pomodoro } = state;

  const stop = async () => setError(await applyCall(api.stopPomodoro()));
  const start = async () => setError(await applyCall(api.startPomodoro(block)));

  if (pomodoro.phase === "idle") {
    return (
      <Card title="Pomodoro">
        <div className="flex items-end gap-4">
          <label className="form-control flex-1">
            <span className="label-text mb-1">Counts towards</span>
            <select
              className="select select-bordered w-full"
              value={block}
              onChange={(e) => setChoice(e.target.value)}
            >
              {blocks.map((b) => (
                <option key={b.name}>{b.name}</option>
              ))}
            </select>
          </label>
          <button
            className="btn btn-primary"
            disabled={!block}
            onClick={() => void start()}
          >
            <Play size={18} aria-hidden="true" />
            Start Pomodoro
          </button>
        </div>
        <ErrorNote message={error} />
      </Card>
    );
  }

  const focusing = pomodoro.phase === "focus";
  const clock = formatClock(pomodoro.remaining_sec);
  return (
    <Card title={focusing ? `Focus: ${pomodoro.block_name}` : "Break"}>
      <div className="flex items-center gap-6">
        {focusing && <Logo breathing className="w-14" />}
        <span
          key={clock}
          role="timer"
          className="animate-fade-in font-serif text-5xl font-medium"
        >
          {clock}
        </span>
        <button className="btn btn-ghost ml-auto" onClick={() => void stop()}>
          <Square size={18} aria-hidden="true" />
          {focusing ? "Stop" : "Skip break"}
        </button>
      </div>
      <p className="text-sm text-neutral">
        {focusing
          ? "Allowance sites are closed until this ends."
          : "Back to work when you're ready."}
      </p>
      <ErrorNote message={error} />
    </Card>
  );
}
