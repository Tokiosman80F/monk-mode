import { useStore } from "../store";
import type { Snapshot } from "../types/Snapshot";

export function makeSnapshot(): Snapshot {
  return {
    state: {
      settings: {
        version: 2,
        setup_done: true,
        allowance_sites: [
          { domain: "youtube.com", daily_min: 30 },
          { domain: "reddit.com", daily_min: 10 },
        ],
        always_blocked: ["tiktok.com"],
        shorts_blocker_enabled: true,
        pomodoro_focus_min: 50,
        pomodoro_break_min: 10,
        daily_focus_target_min: 660,
        profile: {
          name: "Sam",
          focus_areas: [{ name: "React" }, { name: "Products" }],
        },
      },
      lock: null,
      today: {
        date: "2026-11-05",
        blocked_attempts: 0,
        focus_sec: { "Learn hooks": 3600, React: 2400 },
        site_used_sec: { "youtube.com": 120 },
        task_xp: 0,
        tasks_done: 0,
      },
      pomodoro: { phase: "idle" },
      streak_days: 4,
      lock_day: null,
      tasks: [
        {
          id: 1,
          title: "Learn hooks",
          priority: "medium",
          done: false,
          done_on: null,
          allocated_min: 180,
          subtasks: [],
          focus_area: "React",
          carried_from_date: null,
        },
        {
          id: 2,
          title: "Ship the page",
          priority: "high",
          done: false,
          done_on: null,
          allocated_min: 90,
          subtasks: [
            { id: 1, title: "Write it", done: true, allocated_min: 30 },
            { id: 2, title: "Test it", done: false, allocated_min: 0 },
          ],
          focus_area: null,
          carried_from_date: null,
        },
      ],
      small_items: [
        {
          id: 3,
          date: "2026-11-05",
          title: "Call the bank",
          done: false,
          carried_from_date: "2026-11-04",
        },
      ],
      next_id: 4,
      planned: true,
      leftovers: [],
      progress: {
        xp: 320,
        today_xp: 100,
        level: 3,
        level_xp: 20,
        next_level_xp: 300,
        badges: ["first_focus"],
      },
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
