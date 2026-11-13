import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { fillDays, formatDuration, fromDateKey, totalFocusSec } from "../lib";
import { useStore } from "../store";
import type { Stats } from "../types/Stats";
import { Card, ErrorNote, PageTitle } from "../ui";

const RANGES = [7, 30];
const WIDTH = 700;
const HEIGHT = 200;
const AXIS = 24;
/** Room above the tallest bar for its label. */
const TOP = 20;

function Bars({
  days,
  targetSec,
}: {
  days: { date: string; focusSec: number }[];
  targetSec: number;
}) {
  const top = Math.max(targetSec, ...days.map((d) => d.focusSec), 1);
  const slot = WIDTH / days.length;
  const chart = HEIGHT - AXIS - TOP;
  const y = (sec: number) => TOP + chart - (sec / top) * chart;
  const targetY = y(targetSec);
  const labelEvery = days.length > 7 ? 5 : 1;
  // A week has room for each bar's hours. A month shows them on hover.
  const showValues = days.length <= 7;

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      role="img"
      aria-label={`Focus time for the last ${days.length} days`}
      className="w-full"
    >
      <line
        x1={0}
        x2={WIDTH}
        y1={targetY}
        y2={targetY}
        strokeDasharray="4 4"
        className="stroke-neutral"
      />
      <text
        x={WIDTH}
        y={targetY - 6}
        textAnchor="end"
        className="fill-neutral text-[12px]"
      >
        {`Target ${formatDuration(targetSec)}`}
      </text>
      {days.map((day, i) => {
        const height = (day.focusSec / top) * chart;
        const label = showValues
          ? fromDateKey(day.date).toLocaleDateString(undefined, {
              weekday: "short",
              day: "numeric",
            })
          : day.date.slice(8);
        return (
          <g key={day.date}>
            <rect
              data-testid="bar"
              x={i * slot + slot * 0.15}
              y={y(day.focusSec)}
              width={slot * 0.7}
              height={height}
              rx={3}
              className={
                day.focusSec >= targetSec ? "fill-secondary" : "fill-neutral/40"
              }
            >
              <title>{`${day.date}: ${formatDuration(day.focusSec)}`}</title>
            </rect>
            {showValues && day.focusSec >= 60 && (
              <text
                x={i * slot + slot / 2}
                // Inside the top of a tall bar, so it never crosses the target line.
                y={height > 28 ? y(day.focusSec) + 18 : y(day.focusSec) - 6}
                textAnchor="middle"
                className={`text-[12px] font-medium ${height > 28 && day.focusSec >= targetSec ? "fill-secondary-content" : "fill-base-content"}`}
              >
                {formatDuration(day.focusSec)}
              </text>
            )}
            {(days.length - 1 - i) % labelEvery === 0 && (
              <text
                x={i * slot + slot / 2}
                y={HEIGHT - 6}
                textAnchor="middle"
                className="fill-neutral text-[12px]"
              >
                {label}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

export default function StatsScreen() {
  const settings = useStore((s) => s.snapshot?.state.settings);
  // The service day runs 06:00 to 05:59, so at 01:00 it is still yesterday.
  const today = useStore((s) => s.snapshot?.state.today.date);
  const [range, setRange] = useState(7);
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    api
      .getStats(range)
      .then((next) => current && setStats(next))
      .catch((e) => current && setError(errorText(e)));
    return () => {
      current = false;
    };
  }, [range]);

  const focusTotal =
    stats?.days.reduce((sum, day) => sum + totalFocusSec(day), 0) ?? 0;

  return (
    <>
      <PageTitle>Stats</PageTitle>
      <div role="tablist" className="tabs-boxed tabs w-fit bg-base-200">
        {RANGES.map((days) => (
          <button
            key={days}
            role="tab"
            aria-selected={range === days}
            className={`tab ${range === days ? "tab-active" : ""}`}
            onClick={() => setRange(days)}
          >
            {days} days
          </button>
        ))}
      </div>
      <ErrorNote message={error} />
      {stats && (
        <>
          <Card
            title="Focus time"
            action={
              <span className="text-sm text-neutral">
                {formatDuration(focusTotal)} in total,{" "}
                {formatDuration(Math.round(focusTotal / range))} a day
              </span>
            }
          >
            <Bars
              days={fillDays(
                stats.days,
                range,
                today ? fromDateKey(today) : new Date(),
              )}
              targetSec={(settings?.daily_focus_target_min ?? 0) * 60}
            />
            <p className="text-sm text-neutral">
              The dashed line is your daily target. Days that reach it show in
              green.
            </p>
          </Card>
          <Card title="Focus by task">
            <ul className="flex flex-col gap-1">
              {Object.entries(stats.focus_totals_sec).map(([task, sec]) => (
                <li key={task} className="flex justify-between">
                  <span className="truncate">{task}</span>
                  <span className="text-neutral">
                    {formatDuration(sec ?? 0)}
                  </span>
                </li>
              ))}
              <li className="flex justify-between border-t border-base-300 pt-2">
                <span>Tasks finished</span>
                <span className="text-neutral">
                  {stats.days.reduce((sum, day) => sum + day.tasks_done, 0)}
                </span>
              </li>
              <li className="flex justify-between">
                <span>Blocked attempts</span>
                <span className="text-neutral">{stats.blocked_attempts}</span>
              </li>
            </ul>
          </Card>
        </>
      )}
    </>
  );
}
