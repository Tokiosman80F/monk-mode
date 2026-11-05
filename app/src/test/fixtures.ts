import { useStore } from "../store";
import type { Snapshot } from "../types/Snapshot";

export function makeSnapshot(): Snapshot {
  return {
    state: {
      settings: {
        version: 1,
        setup_done: true,
        daily_allowance_min: 10,
        allowance_sites: ["youtube.com", "reddit.com"],
        always_blocked: ["tiktok.com"],
        shorts_blocker_enabled: true,
        study_blocks: [
          { name: "React", target_min: 180 },
          { name: "Building", target_min: 360 },
          { name: "LinkedIn course", target_min: 120 },
        ],
        pomodoro_focus_min: 50,
        pomodoro_break_min: 10,
        daily_focus_target_min: 660,
      },
      lock: null,
      today: {
        date: "2026-11-05",
        allowance_used_sec: 120,
        blocked_attempts: 0,
        focus_sec: { React: 6000 },
      },
      pomodoro: { phase: "idle" },
      streak_days: 4,
      lock_day: null,
    },
    blocked: { hosts_domains: [], url_patterns: [] },
    connected_browsers: [],
    missing_browsers: [],
  };
}

export const FUTURE_LOCK = {
  created_at: "2026-01-01T00:00:00Z",
  ends_at: "2099-01-01T00:00:00Z",
};

export function setStore(change: (snapshot: Snapshot) => void = () => {}) {
  const snapshot = makeSnapshot();
  change(snapshot);
  useStore.setState({ snapshot, connected: true });
  return snapshot;
}
