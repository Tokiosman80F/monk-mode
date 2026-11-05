import { ArrowRight, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import { cleanDomain, isLocked, lockedUntil } from "../lib";
import { applyCall, useStore } from "../store";
import type { Settings } from "../types/Settings";
import { Card, ErrorNote, LockTip, PageTitle } from "../ui";

type ListKey = "allowance_sites" | "always_blocked";

const TABS: [ListKey, string][] = [
  ["allowance_sites", "Allowance sites"],
  ["always_blocked", "Always blocked"],
];

export default function BlockLists() {
  const snapshot = useStore((s) => s.snapshot);
  const [tab, setTab] = useState<ListKey>("allowance_sites");
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!snapshot) return <PageTitle>Block lists</PageTitle>;

  const { settings, lock } = snapshot.state;
  const locked = isLocked(lock);
  const tip = locked ? lockedUntil(lock) : "";
  const save = async (next: Settings) =>
    setError(await applyCall(api.updateSettings(next)));

  const add = async () => {
    const domain = cleanDomain(input);
    if (!domain) return;
    if (settings[tab].includes(domain)) {
      setError(`${domain} is already on this list.`);
      return;
    }
    await save({ ...settings, [tab]: [...settings[tab], domain] });
    setInput("");
  };
  const remove = (site: string) =>
    save({ ...settings, [tab]: settings[tab].filter((s) => s !== site) });
  const moveToBlocked = (site: string) =>
    save({
      ...settings,
      allowance_sites: settings.allowance_sites.filter((s) => s !== site),
      always_blocked: [...settings.always_blocked, site],
    });

  return (
    <>
      <PageTitle>Block lists</PageTitle>
      <div role="tablist" className="tabs-boxed tabs w-fit bg-base-200">
        {TABS.map(([key, label]) => (
          <button
            key={key}
            role="tab"
            aria-selected={tab === key}
            className={`tab ${tab === key ? "tab-active" : ""}`}
            onClick={() => setTab(key)}
          >
            {label}
          </button>
        ))}
      </div>

      <Card>
        <p className="text-sm text-neutral">
          {tab === "allowance_sites"
            ? "These share your daily allowance, then close until midnight."
            : "These are blocked all day, every day."}
        </p>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void add();
          }}
        >
          <input
            className="input input-bordered flex-1"
            placeholder="example.com"
            aria-label="Site to add"
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
          <button className="btn btn-primary" type="submit">
            <Plus size={18} aria-hidden="true" />
            Add
          </button>
        </form>
        <ErrorNote message={error} />

        <ul className="flex flex-col">
          {settings[tab].map((site, i) => (
            <li
              key={site}
              style={{ animationDelay: `${i * 40}ms` }}
              className="flex animate-fade-up items-center gap-2 border-t border-base-300 py-2"
            >
              <span className="flex-1">{site}</span>
              {tab === "allowance_sites" && (
                <button
                  className="btn btn-ghost btn-sm"
                  onClick={() => void moveToBlocked(site)}
                >
                  <ArrowRight size={16} aria-hidden="true" />
                  Move to always blocked
                </button>
              )}
              <LockTip tip={tip}>
                <button
                  className="btn btn-ghost btn-sm"
                  aria-label={`Remove ${site}`}
                  disabled={locked}
                  onClick={() => void remove(site)}
                >
                  <Trash2 size={16} aria-hidden="true" />
                </button>
              </LockTip>
            </li>
          ))}
        </ul>
      </Card>

      <Card title="Shorts and Reels">
        <label className="flex items-center justify-between gap-4">
          <span>Block YouTube Shorts, Instagram Reels and Facebook Watch</span>
          <LockTip tip={tip}>
            <input
              type="checkbox"
              className="toggle toggle-primary"
              aria-label="Block Shorts and Reels"
              checked={settings.shorts_blocker_enabled}
              disabled={locked}
              onChange={(e) =>
                void save({
                  ...settings,
                  shorts_blocker_enabled: e.target.checked,
                })
              }
            />
          </LockTip>
        </label>
      </Card>
    </>
  );
}
