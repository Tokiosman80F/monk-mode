import { Flame } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  browserName,
  greeting,
  formatDuration,
  siteLeftSec,
  sitesByUse,
  totalFocusSec,
} from "../lib";
import { useStore } from "../store";
import { Card, LevelBar, LimitRing, PageTitle } from "../ui";
import Pomodoro from "./Pomodoro";
import Tasks from "./Tasks";

const RINGS_SHOWN = 6;

export default function Today() {
  const snapshot = useStore((s) => s.snapshot);
  if (!snapshot) return <PageTitle>Today</PageTitle>;

  const { state, missing_browsers: missing } = snapshot;
  const sites = sitesByUse(state);
  const hidden = sites.length - RINGS_SHOWN;
  const focusing = state.pomodoro.phase === "focus";
  const focusSec = totalFocusSec(state.today);
  const targetSec = state.settings.daily_focus_target_min * 60;
  const { streak_days: streak, lock_day: lockDay } = state;

  return (
    <>
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <PageTitle>Today</PageTitle>
          <p className="text-neutral">
            {greeting(new Date().getHours())}.
            {lockDay && ` Day ${lockDay.day} of ${lockDay.total}. Keep going.`}
          </p>
        </div>
        <LevelBar progress={state.progress} />
      </header>

      {missing.length > 0 && (
        <p role="status" className="alert alert-warning text-sm">
          {missing.map(browserName).join(" and ")}{" "}
          {missing.length === 1 ? "is" : "are"} open without the extension.
          Sites with limits are blocked there after 30 seconds.
        </p>
      )}

      <Pomodoro />

      <div className="grid items-start gap-5 md:grid-cols-[minmax(0,5fr)_minmax(0,8fr)]">
        <div className="flex flex-col gap-5">
          <Card title="Time left">
            <div className="grid grid-cols-3 gap-x-2 gap-y-4">
              {sites.slice(0, RINGS_SHOWN).map((site) => (
                <LimitRing
                  key={site.domain}
                  domain={site.domain}
                  leftSec={siteLeftSec(state, site)}
                  limitSec={site.daily_min * 60}
                  closed={focusing}
                />
              ))}
            </div>
            {hidden > 0 && (
              <NavLink to="/limits" className="link text-sm text-neutral">
                See all {sites.length} limits
              </NavLink>
            )}
          </Card>

          <Card>
            <div className="flex items-center gap-3">
              <Flame
                size={28}
                aria-hidden="true"
                className={streak > 0 ? "text-primary" : "text-neutral"}
              />
              <div>
                <p className="font-display text-xl font-semibold">
                  {streak} {streak === 1 ? "day" : "days"}
                </p>
                <p className="text-xs text-neutral">Your streak</p>
              </div>
            </div>
            <div>
              <div className="mb-1 flex justify-between text-sm">
                <span>Focus today</span>
                <span className="text-neutral">
                  {formatDuration(focusSec)} / {formatDuration(targetSec)}
                </span>
              </div>
              <progress
                className="progress progress-primary w-full"
                aria-label="Focus today"
                value={focusSec}
                max={Math.max(targetSec, 1)}
              />
            </div>
            <p className="text-sm text-neutral">
              {state.progress.today_xp} XP earned today
            </p>
          </Card>
        </div>

        <Tasks />
      </div>
    </>
  );
}
