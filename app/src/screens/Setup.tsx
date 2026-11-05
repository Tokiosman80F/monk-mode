import { CheckCircle2, FolderOpen, Loader2 } from "lucide-react";
import { useState } from "react";
import { api, errorText } from "../api";
import { browserName } from "../lib";
import { applyCall, useStore } from "../store";
import { Card, ErrorNote, Logo } from "../ui";
import { LockWizard } from "./Lock";

const STEPS = ["Welcome", "Service", "Extension", "Your lists", "Lock"];

function Nav({
  back,
  next,
  nextLabel = "Next",
  disabled = false,
}: {
  back?: () => void;
  next: () => void;
  nextLabel?: string;
  disabled?: boolean;
}) {
  return (
    <div className="flex gap-2">
      {back && (
        <button className="btn btn-ghost" onClick={back}>
          Back
        </button>
      )}
      <button className="btn btn-primary" disabled={disabled} onClick={next}>
        {nextLabel}
      </button>
    </div>
  );
}

function ServiceStep({ back, next }: { back: () => void; next: () => void }) {
  const connected = useStore((s) => s.connected);
  const [error, setError] = useState<string | null>(null);
  const install = async () => {
    try {
      await api.installService();
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <Card title="The background service">
      <p>
        A small Windows service does the blocking, so closing this app
        doesn&apos;t stop it. It usually arrives with the installer.
      </p>
      {connected ? (
        <p className="flex items-center gap-2 text-success">
          <CheckCircle2 size={18} aria-hidden="true" />
          The service is running.
        </p>
      ) : (
        <>
          <p className="flex items-center gap-2 text-neutral">
            <Loader2 size={18} className="animate-spin" aria-hidden="true" />
            Waiting for the service…
          </p>
          <button
            className="btn btn-primary w-fit"
            onClick={() => void install()}
          >
            Install the service
          </button>
        </>
      )}
      <ErrorNote message={error} />
      <Nav back={back} next={next} disabled={!connected} />
    </Card>
  );
}

function ExtensionStep({ back, next }: { back: () => void; next: () => void }) {
  const snapshot = useStore((s) => s.snapshot);
  const [folder, setFolder] = useState("");
  const [error, setError] = useState<string | null>(null);
  const connected = snapshot?.connected_browsers ?? [];
  const missing = snapshot?.missing_browsers ?? [];

  const open = async () => {
    try {
      setFolder(await api.openExtensionFolder());
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };

  return (
    <Card title="The browser extension">
      <p>Add it to every browser you use: Chrome, Edge or Brave.</p>
      <ol className="list-decimal pl-5 text-sm">
        <li>
          Open <code>chrome://extensions</code> (<code>edge://extensions</code>{" "}
          in Edge, <code>brave://extensions</code> in Brave).
        </li>
        <li>Turn on Developer mode.</li>
        <li>Choose Load unpacked, and pick the folder below.</li>
        <li>In the extension&apos;s details, turn on Allow in Incognito.</li>
      </ol>
      <button className="btn btn-ghost w-fit" onClick={() => void open()}>
        <FolderOpen size={18} aria-hidden="true" />
        Open the extension folder
      </button>
      {folder && <code className="break-all text-sm">{folder}</code>}
      <ErrorNote message={error} />

      {connected.length > 0 ? (
        <p className="flex items-center gap-2 text-success">
          <CheckCircle2 size={18} aria-hidden="true" />
          Connected in {connected.map(browserName).join(", ")}.
        </p>
      ) : (
        <p className="flex items-center gap-2 text-neutral">
          <Loader2 size={18} className="animate-spin" aria-hidden="true" />
          Waiting for extension…
        </p>
      )}
      {missing.length > 0 && (
        <p className="text-sm text-neutral">
          Still needed in {missing.map(browserName).join(", ")}.
        </p>
      )}
      <Nav back={back} next={next} disabled={connected.length === 0} />
    </Card>
  );
}

function ListsStep({ back, next }: { back: () => void; next: () => void }) {
  const settings = useStore((s) => s.snapshot?.state.settings);
  if (!settings) return null;
  return (
    <Card title="Your starting lists">
      <p className="text-sm text-neutral">
        You can change these later on the Limits screen.
      </p>
      <div>
        <h3 className="font-medium">Daily limits</h3>
        <p className="text-sm">
          {settings.allowance_sites
            .map((site) => `${site.domain} ${site.daily_min} min`)
            .join(", ")}
        </p>
      </div>
      <div>
        <h3 className="font-medium">Always blocked</h3>
        <p className="text-sm">{settings.always_blocked.join(", ")}</p>
      </div>
      <Nav back={back} next={next} />
    </Card>
  );
}

export default function Setup() {
  const settings = useStore((s) => s.snapshot?.state.settings);
  const [step, setStep] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const finish = async () => {
    if (!settings) return;
    setError(
      await applyCall(api.updateSettings({ ...settings, setup_done: true })),
    );
  };
  const back = () => setStep(step - 1);
  const next = () => setStep(step + 1);

  return (
    <main className="min-h-screen overflow-y-auto bg-base-100 text-base-content">
      <div className="mx-auto flex max-w-[760px] flex-col gap-6 p-6">
        <div className="flex items-center gap-2">
          <Logo />
          <span className="font-display text-xl font-semibold">Monk Mode</span>
        </div>
        <ul className="steps">
          {STEPS.map((label, i) => (
            <li
              key={label}
              className={`step ${step >= i ? "step-primary" : ""}`}
            >
              {label}
            </li>
          ))}
        </ul>

        {step === 0 && (
          <Card title="Welcome">
            <p>
              A daily limit for each distracting site, then it closes until
              midnight. The worst ones stay closed. A lock keeps you to it on a
              bad day.
            </p>
            <p>A few steps, then back to work.</p>
            <Nav next={next} nextLabel="Get started" />
          </Card>
        )}
        {step === 1 && <ServiceStep back={back} next={next} />}
        {step === 2 && <ExtensionStep back={back} next={next} />}
        {step === 3 && <ListsStep back={back} next={next} />}
        {step === 4 && (
          <>
            <LockWizard onDone={() => void finish()} />
            <ErrorNote message={error} />
            <div className="flex gap-2">
              <button className="btn btn-ghost" onClick={back}>
                Back
              </button>
              <button className="btn btn-ghost" onClick={() => void finish()}>
                Skip for now
              </button>
            </div>
          </>
        )}
      </div>
    </main>
  );
}
