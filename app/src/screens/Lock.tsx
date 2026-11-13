import { useState } from "react";
import { api } from "../api";
import { LOCK_PHRASE, addDays, addMonths, formatDate, isLocked } from "../lib";
import { applyCall, useStore } from "../store";
import { Card, ErrorNote, PageTitle } from "../ui";

type Choice = "week" | "month" | "three" | "custom";

const CHOICES: [Choice, string][] = [
  ["week", "1 week"],
  ["month", "1 month"],
  ["three", "3 months"],
  ["custom", "Custom date"],
];

function endDate(choice: Choice, custom: string, from: Date): Date | null {
  switch (choice) {
    case "week":
      return addDays(from, 7);
    case "month":
      return addMonths(from, 1);
    case "three":
      return addMonths(from, 3);
    case "custom": {
      // Local midnight at the end of the chosen day.
      const date = custom ? new Date(`${custom}T23:59:59`) : null;
      return date && date > from ? date : null;
    }
  }
}

/**
 * The 3-step lock wizard. Also a step of first-run setup, which passes `onBack` and `onSkip` so
 * there's one set of buttons.
 */
export function LockWizard({
  onDone,
  onBack,
  onSkip,
}: {
  onDone?: () => void;
  onBack?: () => void;
  onSkip?: () => void;
}) {
  const [step, setStep] = useState(1);
  // Short by default: a first lock should be easy to say yes to.
  const [choice, setChoice] = useState<Choice>("week");
  const [custom, setCustom] = useState("");
  const [phrase, setPhrase] = useState("");
  const [error, setError] = useState<string | null>(null);
  const end = endDate(choice, custom, new Date());

  const commit = async () => {
    if (!end) return;
    const failure = await applyCall(api.startLock(end.toISOString()));
    setError(failure);
    if (!failure) onDone?.();
  };

  return (
    <Card
      title="Start a commitment lock"
      action={
        <span className="text-sm text-neutral">
          {step} of 3: {["How long", "The rules", "Commit"][step - 1]}
        </span>
      }
    >
      {step === 1 && (
        <>
          <div className="flex flex-col gap-2">
            {CHOICES.map(([value, label]) => (
              <label key={value} className="flex items-center gap-2">
                <input
                  type="radio"
                  name="duration"
                  className="radio radio-primary"
                  checked={choice === value}
                  onChange={() => setChoice(value)}
                />
                {label}
              </label>
            ))}
          </div>
          {choice === "custom" && (
            <input
              type="date"
              aria-label="Lock end date"
              className="input input-bordered w-fit"
              value={custom}
              onChange={(e) => setCustom(e.target.value)}
            />
          )}
          <p className="text-sm text-neutral">
            Start short. A week is enough to see how it feels.
          </p>
          <div className="flex gap-2">
            {onBack && (
              <button className="btn btn-ghost" onClick={onBack}>
                Back
              </button>
            )}
            <button
              className="btn btn-primary"
              disabled={!end}
              onClick={() => setStep(2)}
            >
              Next
            </button>
            {onSkip && (
              <button className="btn btn-ghost text-neutral" onClick={onSkip}>
                Skip for now
              </button>
            )}
          </div>
        </>
      )}

      {step === 2 && end && (
        <>
          <p>
            Until <strong>{formatDate(end.toISOString())}</strong>, your rules
            can only get stricter.
          </p>
          <div>
            <p className="font-medium">You can still:</p>
            <ul className="list-disc pl-5 text-sm">
              <li>Add sites to either list</li>
              <li>Move a site to always blocked</li>
              <li>Lower any site&apos;s daily limit</li>
            </ul>
          </div>
          <div>
            <p className="font-medium">You won&apos;t be able to:</p>
            <ul className="list-disc pl-5 text-sm">
              <li>Remove any site, or give an always blocked site a limit</li>
              <li>Raise any site&apos;s daily limit</li>
              <li>Turn off the Shorts and Reels blocker</li>
              <li>Shorten or remove the lock</li>
              <li>Stop the service or uninstall the app</li>
            </ul>
          </div>
          <div className="flex gap-2">
            <button className="btn btn-ghost" onClick={() => setStep(1)}>
              Back
            </button>
            <button className="btn btn-primary" onClick={() => setStep(3)}>
              Next
            </button>
          </div>
        </>
      )}

      {step === 3 && (
        <>
          <label className="form-control">
            <span className="label-text mb-1">
              Type <strong>{LOCK_PHRASE}</strong> to confirm.
            </span>
            <input
              className="input input-bordered"
              aria-label="Commitment phrase"
              autoComplete="off"
              value={phrase}
              onChange={(e) => setPhrase(e.target.value)}
            />
          </label>
          <ErrorNote message={error} />
          <div className="flex gap-2">
            <button className="btn btn-ghost" onClick={() => setStep(2)}>
              Back
            </button>
            <button
              className="btn btn-primary"
              disabled={phrase !== LOCK_PHRASE}
              onClick={() => void commit()}
            >
              Start the lock
            </button>
          </div>
        </>
      )}
    </Card>
  );
}

export default function LockScreen() {
  const state = useStore((s) => s.snapshot?.state);
  if (!state) return <PageTitle>Lock</PageTitle>;

  return (
    <>
      <PageTitle>Lock</PageTitle>
      {isLocked(state.lock) && state.lock ? (
        <Card title="Locked">
          <p>
            Your rules can only get stricter until{" "}
            <strong>{formatDate(state.lock.ends_at)}</strong>.
          </p>
          {state.lock_day && (
            <p className="text-neutral">
              Day {state.lock_day.day} of {state.lock_day.total}. Keep going.
            </p>
          )}
        </Card>
      ) : (
        <LockWizard />
      )}
    </>
  );
}
