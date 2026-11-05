import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { fillDays, formatDuration } from "../lib";
import { useStore } from "../store";
import type { Stats } from "../types/Stats";
import { Card, ErrorNote, PageTitle } from "../ui";

const RANGES = [7, 30];
const WIDTH = 700;
const HEIGHT = 200;
const AXIS = 24;

function Bars({
  days,
  targetSec,
}: {
  days: { date: string; focusSec: number }[];
  targetSec: number;
}) {
  const top = Math.max(targetSec, ...days.map((d) => d.focusSec), 1);
  const slot = WIDTH / days.length;
  const chart = HEIGHT - AXIS;
  const targetY = chart - (targetSec / top) * chart;
  const labelEvery = days.length > 7 ? 5 : 1;

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
      {days.map((day, i) => {
        const height = (day.focusSec / top) * chart;
        return (
          <g key={day.date}>
            <rect
              data-testid="bar"
              x={i * slot + slot * 0.15}
              y={chart - height}
              width={slot * 0.7}
              height={height}
              rx={3}
              className={
                day.focusSec >= targetSec ? "fill-secondary" : "fill-neutral/40"
              }
            >
              <title>{`${day.date}: ${formatDuration(day.focusSec)}`}</title>
            </rect>
            {(days.length - 1 - i) % labelEvery === 0 && (
              <text
                x={i * slot + slot / 2}
                y={HEIGHT - 6}
                textAnchor="middle"
                className="fill-neutral text-[12px]"
              >
                {day.date.slice(8)}
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
          <Card title="Focus time">
            <Bars
              days={fillDays(stats.days, range, new Date())}
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
