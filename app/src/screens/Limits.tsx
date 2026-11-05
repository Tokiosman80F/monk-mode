import { ArrowRight, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { api } from "../api";
import {
  cleanDomain,
  formatDuration,
  formatLeft,
  isLocked,
  lockedUntil,
  siteLeftSec,
  siteUsedSec,
} from "../lib";
import { applyCall, useStore } from "../store";
import type { AllowanceSite } from "../types/AllowanceSite";
import type { Settings } from "../types/Settings";
import { Card, ErrorNote, LockTip, PageTitle } from "../ui";

type Tab = "limits" | "blocked";

const TABS: [Tab, string][] = [
  ["limits", "Daily limits"],
  ["blocked", "Always blocked"],
];
const MAX_LIMIT_MIN = 180;
const NEW_SITE_MIN = 10;
/** Matches the service's first warning. */
const LOW_TIME_SEC = 120;

/** Saves when the field is left, so typing "30" doesn't save "3" along the way. */
function MinutesField({
  site,
  max,
  onSave,
}: {
  site: AllowanceSite;
  max: number;
  onSave: (minutes: number) => void;
}) {
  const [draft, setDraft] = useState(String(site.daily_min));
  const commit = () => {
    const minutes = Number(draft);
    if (draft !== "" && Number.isInteger(minutes) && minutes !== site.daily_min)
      onSave(minutes);
    else setDraft(String(site.daily_min));
  };
  return (
    <label className="flex items-center gap-2 text-sm text-neutral">
      <input
        type="number"
        className="input input-sm input-bordered w-20 text-base-content"
        aria-label={`Daily minutes for ${site.domain}`}
        min={0}
        max={max}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
        }}
      />
      min a day
    </label>
  );
}

export default function Limits() {
  const snapshot = useStore((s) => s.snapshot);
  const [tab, setTab] = useState<Tab>("limits");
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!snapshot) return <PageTitle>Limits</PageTitle>;

  const { state } = snapshot;
  const { settings, lock } = state;
  const locked = isLocked(lock);
  const tip = locked ? lockedUntil(lock) : "";
  const sites = settings.allowance_sites;
  const save = async (next: Settings) =>
    setError(await applyCall(api.updateSettings(next)));

  const add = async () => {
    const domain = cleanDomain(input);
    if (!domain) return;
    const taken =
      tab === "limits"
        ? sites.some((site) => site.domain === domain)
        : settings.always_blocked.includes(domain);
    if (taken) {
      setError(`${domain} is already on this list.`);
      return;
    }
    await save(
      tab === "limits"
        ? {
            ...settings,
            allowance_sites: [...sites, { domain, daily_min: NEW_SITE_MIN }],
          }
        : {
            ...settings,
            always_blocked: [...settings.always_blocked, domain],
          },
    );
    setInput("");
  };
  const setMinutes = (domain: string, daily_min: number) =>
    save({
      ...settings,
      allowance_sites: sites.map((site) =>
        site.domain === domain ? { ...site, daily_min } : site,
      ),
    });
  const removeSite = (domain: string) =>
    save({
      ...settings,
      allowance_sites: sites.filter((site) => site.domain !== domain),
    });
  const removeBlocked = (domain: string) =>
    save({
      ...settings,
      always_blocked: settings.always_blocked.filter((d) => d !== domain),
    });
  const moveToBlocked = (domain: string) =>
    save({
      ...settings,
      allowance_sites: sites.filter((site) => site.domain !== domain),
      always_blocked: [...settings.always_blocked, domain],
    });

  return (
    <>
      <PageTitle>Limits</PageTitle>
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
          {tab === "limits"
            ? "Each site gets its own minutes, up to 180 a day, then closes until midnight. A subdomain also counts towards its main site."
            : "These are blocked all day, every day."}
          {locked && tab === "limits" && ` ${tip}: you can only lower these.`}
        </p>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void add();
          }}
        >
          <input
            className="input input-bordered min-w-0 flex-1"
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

        {tab === "limits" ? (
          <ul className="flex flex-col">
            {sites.map((site) => {
              const used = siteUsedSec(state, site.domain);
              const left = siteLeftSec(state, site);
              const running = left > 0 && left <= LOW_TIME_SEC;
              return (
                <li
                  key={`${site.domain}-${site.daily_min}`}
                  className="flex flex-wrap items-center gap-3 border-t border-base-300 py-3"
                >
                  <div className="min-w-40 flex-1">
                    <p className="truncate">{site.domain}</p>
                    <p className="text-xs text-neutral">
                      {formatDuration(used)} used
                    </p>
                    <progress
                      className={`progress mt-1 block h-1 w-full max-w-48 ${running ? "progress-error" : "progress-secondary"}`}
                      aria-label={`Time left on ${site.domain}`}
                      value={left}
                      max={Math.max(site.daily_min * 60, 1)}
                    />
                  </div>
                  <p
                    className={`w-24 text-right font-display text-lg font-medium tabular-nums ${
                      left === 0
                        ? "text-neutral"
                        : running
                          ? "text-error"
                          : "text-secondary"
                    }`}
                  >
                    {formatLeft(left)}
                    {left > 0 && (
                      <span className="ml-1 text-xs font-normal text-neutral">
                        left
                      </span>
                    )}
                  </p>
                  <MinutesField
                    site={site}
                    max={locked ? site.daily_min : MAX_LIMIT_MIN}
                    onSave={(minutes) => void setMinutes(site.domain, minutes)}
                  />
                  <button
                    className="btn btn-ghost btn-sm"
                    onClick={() => void moveToBlocked(site.domain)}
                  >
                    <ArrowRight size={16} aria-hidden="true" />
                    Always block
                  </button>
                  <LockTip tip={tip}>
                    <button
                      className="btn btn-ghost btn-sm"
                      aria-label={`Remove ${site.domain}`}
                      disabled={locked}
                      onClick={() => void removeSite(site.domain)}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                    </button>
                  </LockTip>
                </li>
              );
            })}
          </ul>
        ) : (
          <ul className="flex flex-col">
            {settings.always_blocked.map((domain) => (
              <li
                key={domain}
                className="flex items-center gap-2 border-t border-base-300 py-2"
              >
                <span className="flex-1">{domain}</span>
                <LockTip tip={tip}>
                  <button
                    className="btn btn-ghost btn-sm"
                    aria-label={`Remove ${domain}`}
                    disabled={locked}
                    onClick={() => void removeBlocked(domain)}
                  >
                    <Trash2 size={16} aria-hidden="true" />
                  </button>
                </LockTip>
              </li>
            ))}
          </ul>
        )}
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
