import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../api";
import { LOCK_PHRASE } from "../lib";
import { useStore } from "../store";
import { FUTURE_LOCK, makeSnapshot, setStore } from "../test/fixtures";
import About from "./About";
import FocusSettings from "./FocusSettings";
import Limits from "./Limits";
import Mini from "./Mini";
import LockScreen from "./Lock";
import Pomodoro from "./Pomodoro";
import Rewards from "./Rewards";
import Setup from "./Setup";
import StatsScreen from "./Stats";
import MorningPrompt from "./MorningPrompt";
import PlannerSetup from "./PlannerSetup";
import SmallWork from "./SmallWork";
import Tasks from "./Tasks";
import type { Snapshot } from "../types/Snapshot";
import Today from "./Today";

vi.mock("../api", () => ({
  errorText: (e: { message?: string }) => e.message ?? String(e),
  api: {
    getState: vi.fn(),
    updateSettings: vi.fn(),
    updateTasks: vi.fn(),
    startLock: vi.fn(),
    startPomodoro: vi.fn(),
    stopPomodoro: vi.fn(),
    updateSmallItems: vi.fn(),
    resolveLeftover: vi.fn(),
    planDay: vi.fn(),
    pausePomodoro: vi.fn(),
    getStats: vi.fn(),
    installService: vi.fn(),
    openExtensionFolder: vi.fn(),
    dismissMini: vi.fn(),
    onStateChanged: vi.fn(async () => () => {}),
    onServiceStatus: vi.fn(async () => () => {}),
  },
}));

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  vi.mocked(api.updateSettings).mockResolvedValue(makeSnapshot());
  vi.mocked(api.updateTasks).mockResolvedValue(makeSnapshot());
  vi.mocked(api.startPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.stopPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.updateSmallItems).mockResolvedValue(makeSnapshot());
  vi.mocked(api.resolveLeftover).mockResolvedValue(makeSnapshot());
  vi.mocked(api.planDay).mockResolvedValue(makeSnapshot());
  vi.mocked(api.pausePomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.startLock).mockResolvedValue(makeSnapshot());
});

function savedSettings() {
  return vi.mocked(api.updateSettings).mock.calls[0][0];
}

function savedTasks() {
  return vi.mocked(api.updateTasks).mock.calls[0][0];
}

function fillForm(title: string, time: string) {
  fireEvent.change(screen.getByLabelText("New task"), {
    target: { value: title },
  });
  fireEvent.change(screen.getByLabelText("Total time"), {
    target: { value: time },
  });
}

describe("Today", () => {
  it("shows the date, a quote, tasks, level, streak and focus", () => {
    setStore();
    render(<Today />);
    const year = String(new Date().getFullYear());
    expect(
      screen.getByRole("heading", { level: 1, name: new RegExp(year) }),
    ).toBeInTheDocument();
    expect(screen.getByRole("blockquote")).toBeInTheDocument();
    expect(screen.getByLabelText("Finish Ship the page")).toBeInTheDocument();
    expect(screen.getByLabelText("Level 3")).toBeInTheDocument();
    expect(screen.getByText("20 / 300 XP")).toBeInTheDocument();
    expect(screen.getByText("4 days")).toBeInTheDocument();
    expect(screen.getByText("1h 40m / 11h")).toBeInTheDocument();
    expect(
      screen.getByText("4h 30m planned in open tasks"),
    ).toBeInTheDocument();
  });

  it("has no time-left section", () => {
    setStore();
    render(<Today />);
    expect(screen.queryByText("Time left")).not.toBeInTheDocument();
    expect(screen.queryByText("youtube")).not.toBeInTheDocument();
  });

  it("shows the day counter while locked", () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
      state.lock_day = { day: 12, total: 90 };
    });
    render(<Today />);
    expect(screen.getByText("Day 12 of 90")).toBeInTheDocument();
  });

  it("explains a browser without the extension", () => {
    setStore((s) => {
      s.missing_browsers = ["msedge.exe"];
    });
    render(<Today />);
    expect(screen.getByRole("status")).toHaveTextContent(
      "Edge is open without the extension",
    );
  });
});

describe("Tasks", () => {
  it("lists tasks with the most important first, and no matrix", () => {
    setStore();
    render(<Tasks />);
    const boxes = screen.getAllByRole("checkbox", { name: /^Finish / });
    expect(boxes[0]).toHaveAccessibleName("Finish Ship the page");
    expect(boxes[1]).toHaveAccessibleName("Finish Learn hooks");
    expect(
      screen.getByRole("button", { name: /^Time for Ship the page: 1h 30m/ }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Do first")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Urgent" })).toBeNull();
  });

  it("adds a task with its priority and total time", async () => {
    setStore();
    render(<Tasks />);
    fillForm("  Read docs  ", "3h");
    fireEvent.click(screen.getByRole("button", { name: "High" }));
    expect(screen.getByRole("button", { name: "High" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() =>
      expect(savedTasks()[2]).toMatchObject({
        id: 4,
        title: "Read docs",
        allocated_min: 180,
        priority: "high",
      }),
    );
  });

  it("uses medium priority unless told otherwise", async () => {
    setStore();
    render(<Tasks />);
    fillForm("Read", "1h");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(savedTasks()[2].priority).toBe("medium"));
  });

  it("opens a new task so its subtasks can be added next", async () => {
    const snapshot = setStore();
    const next = structuredClone(snapshot);
    next.state.tasks.push({
      id: 4,
      title: "Read",
      priority: "medium",
      done: false,
      done_on: null,
      allocated_min: 60,
      subtasks: [],
      focus_area: null,
      carried_from_date: null,
    });
    vi.mocked(api.updateTasks).mockResolvedValue(next);
    render(<Tasks />);
    fillForm("Read", "1h");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(
      await screen.findByLabelText("Add a subtask to Read"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Add a subtask to Learn hooks"),
    ).not.toBeInTheDocument();
  });

  it("needs a time for a new task", () => {
    setStore();
    render(<Tasks />);
    fillForm("Read", "");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(screen.getByRole("alert")).toHaveTextContent("How long");
    fillForm("Read", "soon");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(screen.getByRole("alert")).toHaveTextContent("How long");
    expect(api.updateTasks).not.toHaveBeenCalled();
  });

  it("skips a blank task", () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(api.updateTasks).not.toHaveBeenCalled();
  });

  it("removes a task", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(screen.getByRole("button", { name: "Remove Learn hooks" }));
    await waitFor(() => expect(savedTasks().map((t) => t.id)).toEqual([2]));
  });

  it("finishing a task saves it and shows the XP", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(screen.getByLabelText("Finish Ship the page"));
    expect(screen.getByText("+50 XP")).toBeInTheDocument();
    await waitFor(() => expect(savedTasks()[1].done).toBe(true));
  });

  it("cycles a task's priority", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(screen.getByLabelText(/^Priority of Learn hooks/));
    await waitFor(() => expect(savedTasks()[0].priority).toBe("low"));
  });

  it("shows time spent against the time set, and edits it", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: /^Time for Learn hooks: 1h \/ 3h/ }),
    );
    const field = screen.getByLabelText("Time for Learn hooks");
    fireEvent.change(field, { target: { value: "2h 30m" } });
    fireEvent.blur(field);
    await waitFor(() => expect(savedTasks()[0].allocated_min).toBe(150));
  });

  it("ignores a time it can't read", () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: /^Time for Learn hooks/ }),
    );
    const field = screen.getByLabelText("Time for Learn hooks");
    fireEvent.change(field, { target: { value: "soon" } });
    fireEvent.blur(field);
    expect(api.updateTasks).not.toHaveBeenCalled();
  });

  it("opens subtasks, adds one with time, finishes and removes them", async () => {
    setStore();
    render(<Tasks />);
    expect(screen.getByText("1/2")).toBeInTheDocument();
    expect(screen.queryByText("Test it")).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Show subtasks of Ship the page" }),
    );
    expect(screen.getByText("Test it")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Add a subtask to Ship the page"), {
      target: { value: "Deploy it" },
    });
    fireEvent.change(
      screen.getByLabelText("Time for the new subtask of Ship the page"),
      { target: { value: "20m" } },
    );
    fireEvent.submit(
      screen.getByLabelText("Add a subtask to Ship the page").closest("form")!,
    );
    await waitFor(() =>
      expect(savedTasks()[1].subtasks[2]).toMatchObject({
        title: "Deploy it",
        allocated_min: 20,
      }),
    );

    vi.mocked(api.updateTasks).mockClear();
    fireEvent.click(screen.getByLabelText("Finish Test it"));
    await waitFor(() => expect(savedTasks()[1].done).toBe(true));

    vi.mocked(api.updateTasks).mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Remove Write it" }));
    await waitFor(() => expect(savedTasks()[1].subtasks).toHaveLength(1));
  });

  it("edits a subtask's time", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: "Show subtasks of Ship the page" }),
    );
    fireEvent.click(screen.getByRole("button", { name: /^Time for Test it/ }));
    const field = screen.getByLabelText("Time for Test it");
    fireEvent.change(field, { target: { value: "45m" } });
    fireEvent.blur(field);
    await waitFor(() =>
      expect(savedTasks()[1].subtasks[1].allocated_min).toBe(45),
    );
  });

  it("warns when subtasks go over the task's time", () => {
    setStore(({ state }) => {
      state.tasks[1].allocated_min = 10;
    });
    render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: "Show subtasks of Ship the page" }),
    );
    expect(screen.getByText(/20m more than this task/)).toBeInTheDocument();
  });

  it("starts a focus session on a task, unless one is running", async () => {
    setStore();
    const { rerender } = render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: "Focus on Learn hooks" }),
    );
    await waitFor(() => expect(api.startPomodoro).toHaveBeenCalledWith(1));

    setStore(({ state }) => {
      state.pomodoro = { phase: "break", remaining_sec: 60 };
    });
    rerender(<Tasks />);
    expect(
      screen.getByRole("button", { name: "Focus on Learn hooks" }),
    ).toBeDisabled();
  });

  it("shows a refusal from the service", async () => {
    setStore();
    vi.mocked(api.updateTasks).mockRejectedValue({ message: "Too long." });
    render(<Tasks />);
    fireEvent.click(screen.getByLabelText("Finish Learn hooks"));
    expect(await screen.findByRole("alert")).toHaveTextContent("Too long.");
  });
});

describe("Mini timer", () => {
  it("shows only the time left", () => {
    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "Building project",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 49 * 60,
        paused: false,
      };
    });
    render(<Mini />);
    expect(
      screen.getByRole("timer", { name: "Focus time left" }),
    ).toHaveTextContent(/^49 min left$/);
    expect(screen.queryByText(/Building project/)).not.toBeInTheDocument();
  });

  it("shows a break, and can be hidden", () => {
    setStore(({ state }) => {
      state.pomodoro = { phase: "break", remaining_sec: 45 };
    });
    render(<Mini />);
    expect(
      screen.getByRole("timer", { name: "Break time left" }),
    ).toHaveTextContent("45 sec left");
    fireEvent.click(screen.getByRole("button", { name: "Hide the timer" }));
    expect(api.dismissMini).toHaveBeenCalled();
  });
});

describe("Rewards", () => {
  it("shows the level, XP to go and which badges are earned", () => {
    setStore();
    render(<Rewards />);
    expect(screen.getByRole("heading", { name: "Level 3" })).toBeVisible();
    expect(
      screen.getByText("280 XP to level 4. 320 XP in total."),
    ).toBeInTheDocument();
    expect(screen.getAllByTestId("earned")).toHaveLength(1);
    expect(screen.getAllByTestId("unearned")).toHaveLength(8);
    expect(screen.getByText("Badges: 1 of 9")).toBeInTheDocument();
  });
});

describe("Pomodoro", () => {
  it("starts on the most pressing open task", async () => {
    setStore();
    render(<Pomodoro />);
    expect(screen.getByText(/Ship the page/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Start focus/ }));
    await waitFor(() => expect(api.startPomodoro).toHaveBeenCalledWith(2));
  });

  it("offers free focus, and needs a task for the main button", async () => {
    setStore(({ state }) => {
      state.tasks = [];
    });
    render(<Pomodoro />);
    expect(screen.getByRole("button", { name: /Start focus/ })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Free focus" }));
    await waitFor(() => expect(api.startPomodoro).toHaveBeenCalledWith(null));
  });

  it("carries on with the task focused on before the break", async () => {
    // The low-priority task was the last focus, so it beats the high-priority one.
    const focusing = makeSnapshot();
    focusing.state.pomodoro = {
      phase: "focus",
      task_title: "Learn hooks",
      started_at: "2026-11-05T09:00:00Z",
      remaining_sec: 1,
      paused: false,
    };
    // Through the store, as a push from the service arrives.
    useStore.getState().setSnapshot(focusing);
    setStore();
    render(<Pomodoro />);
    expect(screen.getByText("Carry on with:")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Start focus/ }));
    await waitFor(() => expect(api.startPomodoro).toHaveBeenCalledWith(1));
    localStorage.clear();
  });

  it("counts down from what the service pushes, and can stop", async () => {
    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "React",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 125,
        paused: false,
      };
    });
    const { rerender } = render(<Pomodoro />);
    expect(screen.getByRole("timer")).toHaveTextContent("2:05");
    expect(screen.getByText(/^Ends at /)).toBeInTheDocument();
    expect(screen.getByText("Focus: React")).toBeInTheDocument();
    expect(screen.getByText("Session 1 of 1")).toBeInTheDocument();

    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "React",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 124,
        paused: false,
      };
    });
    rerender(<Pomodoro />);
    expect(screen.getByRole("timer")).toHaveTextContent("2:04");

    fireEvent.click(screen.getByRole("button", { name: /Stop/ }));
    await waitFor(() => expect(api.stopPomodoro).toHaveBeenCalled());
  });

  it("shows a segment for each session of the task, and can pause", async () => {
    setStore(({ state }) => {
      state.settings.pomodoro_focus_min = 50;
      state.tasks = [
        {
          id: 1,
          title: "Project building",
          priority: "high",
          done: false,
          done_on: null,
          allocated_min: 600,
          subtasks: [],
          focus_area: null,
          carried_from_date: null,
        },
      ];
      state.today.focus_sec = { "Project building": 2 * 3000 + 1200 };
      state.pomodoro = {
        phase: "focus",
        task_title: "Project building",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 1800,
        paused: false,
      };
    });
    render(<Pomodoro />);
    expect(screen.getByText("Session 3 of 12")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(api.pausePomodoro).toHaveBeenCalledWith(true));
  });

  it("offers to resume a paused session", () => {
    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "React",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 600,
        paused: true,
      };
    });
    render(<Pomodoro />);
    expect(screen.getByRole("button", { name: "Resume" })).toBeInTheDocument();
    expect(screen.queryByText(/^Ends at/)).not.toBeInTheDocument();
  });

  it("offers to skip a break", () => {
    setStore(({ state }) => {
      state.pomodoro = { phase: "break", remaining_sec: 60 };
    });
    render(<Pomodoro />);
    expect(
      screen.getByRole("button", { name: /Skip break/ }),
    ).toBeInTheDocument();
  });

  it("shows a refusal from the service", async () => {
    setStore();
    vi.mocked(api.startPomodoro).mockRejectedValue({ message: "Nope." });
    render(<Pomodoro />);
    fireEvent.click(screen.getByRole("button", { name: /Start focus/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Nope.");
  });
});

describe("Planner setup", () => {
  it("saves a profile with the study hours as the daily target", async () => {
    setStore(({ state }) => {
      state.settings.profile = null;
    });
    render(<PlannerSetup />);
    fireEvent.click(screen.getByRole("button", { name: /Continue/ }));
    expect(screen.getByRole("alert")).toHaveTextContent("Enter your name");
    expect(api.updateSettings).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Your name"), {
      target: { value: "Sam" },
    });
    fireEvent.change(screen.getByLabelText("Daily study hours"), {
      target: { value: "10.5" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Remove focus area 3" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Monday" }));
    fireEvent.click(screen.getByRole("button", { name: /Continue/ }));
    await waitFor(() => expect(api.updateSettings).toHaveBeenCalled());
    const saved = savedSettings();
    expect(saved.daily_focus_target_min).toBe(630);
    expect(saved.profile).toEqual({
      name: "Sam",
      focus_areas: [
        { name: "React", target_min: 180 },
        { name: "Products", target_min: 360 },
      ],
      week_start: "monday",
    });
  });
});

describe("Morning prompt", () => {
  function unplanned(change: (state: Snapshot["state"]) => void = () => {}) {
    setStore(({ state }) => {
      state.planned = false;
      state.tasks = [];
      change(state);
    });
  }

  it("can't start the day with no tasks, even with small work", async () => {
    unplanned();
    render(<MorningPrompt />);
    expect(screen.getByText("Call the bank")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Start my day/ }));
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Write at least one task",
    );
    expect(api.planDay).not.toHaveBeenCalled();
  });

  it("adds a task with a focus area and an optional estimate", async () => {
    unplanned();
    render(<MorningPrompt />);
    fireEvent.change(screen.getByLabelText("New task"), {
      target: { value: "Hooks chapter" },
    });
    fireEvent.change(screen.getByLabelText("Focus area"), {
      target: { value: "Products" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add task" }));
    await waitFor(() => expect(api.updateTasks).toHaveBeenCalled());
    expect(savedTasks()).toEqual([
      expect.objectContaining({
        title: "Hooks chapter",
        focus_area: "Products",
        allocated_min: 0,
        id: 4,
      }),
    ]);
  });

  it("blocks a 5th task with a clear message", () => {
    unplanned((state) => {
      state.tasks = [1, 2, 3, 4].map((id) => ({
        id,
        title: `Task ${id}`,
        priority: "low" as const,
        done: false,
        done_on: null,
        allocated_min: 0,
        subtasks: [],
        focus_area: null,
        carried_from_date: null,
      }));
    });
    render(<MorningPrompt />);
    expect(screen.queryByLabelText("New task")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("4 tasks");
  });

  it("shows today's target and offers yesterday's unfinished tasks", async () => {
    unplanned((state) => {
      state.today_target = "Finish the hooks chapter";
      state.leftovers = [
        {
          id: 9,
          title: "Recipe app",
          priority: "high",
          done: false,
          done_on: null,
          allocated_min: 180,
          subtasks: [],
          focus_area: "Products",
          carried_from_date: null,
        },
      ];
    });
    // The service answers with the leftover still there, so both buttons can be tried.
    vi.mocked(api.resolveLeftover).mockImplementation(
      async () => useStore.getState().snapshot!,
    );
    render(<MorningPrompt />);
    expect(screen.getByText("Finish the hooks chapter")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Carry Recipe app to today" }),
    );
    await waitFor(() =>
      expect(api.resolveLeftover).toHaveBeenCalledWith(9, { kind: "carry" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Move Recipe app to another day" }),
    );
    const day = screen.getByLabelText("Day to move Recipe app to");
    expect(day).toHaveAttribute("min", "2026-11-06");
    fireEvent.change(day, { target: { value: "2026-11-09" } });
    fireEvent.click(screen.getByRole("button", { name: "Move" }));
    await waitFor(() =>
      expect(api.resolveLeftover).toHaveBeenCalledWith(9, {
        kind: "move",
        date: "2026-11-09",
      }),
    );
  });

  it("starts the day once there's a task", async () => {
    unplanned((state) => {
      state.tasks = makeSnapshot().state.tasks;
    });
    render(<MorningPrompt />);
    fireEvent.click(screen.getByRole("button", { name: /Start my day/ }));
    await waitFor(() => expect(api.planDay).toHaveBeenCalled());
  });
});

describe("Task limit", () => {
  it("stops offering new tasks at 4", () => {
    setStore(({ state }) => {
      state.tasks = [1, 2, 3, 4].map((id) => ({
        id,
        title: `Task ${id}`,
        priority: "low" as const,
        done: false,
        done_on: null,
        allocated_min: 60,
        subtasks: [],
        focus_area: null,
        carried_from_date: null,
      }));
    });
    render(<Tasks />);
    expect(screen.queryByLabelText("New task")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("4 tasks");
  });

  it("gives a new task an id the archive hasn't used", async () => {
    setStore(({ state }) => {
      state.next_id = 50;
    });
    render(<Tasks />);
    fireEvent.change(screen.getByLabelText("New task"), {
      target: { value: "Write" },
    });
    fireEvent.change(screen.getByLabelText("Total time"), {
      target: { value: "1h" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(api.updateTasks).toHaveBeenCalled());
    expect(savedTasks().at(-1)?.id).toBe(50);
  });
});

describe("Small work", () => {
  it("adds, ticks and removes items, and shows where one came from", async () => {
    setStore();
    render(<SmallWork />);
    expect(screen.getByText(/^from /)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("New small work"), {
      target: { value: "Email Sara" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(api.updateSmallItems).toHaveBeenCalled());
    const added = vi.mocked(api.updateSmallItems).mock.calls[0][0];
    expect(added.map((i) => i.title)).toEqual(["Call the bank", "Email Sara"]);
    expect(added[1].id).toBe(4);

    vi.mocked(api.updateSmallItems).mockClear();
    fireEvent.click(screen.getByLabelText("Finish Call the bank"));
    await waitFor(() => expect(api.updateSmallItems).toHaveBeenCalled());
    expect(vi.mocked(api.updateSmallItems).mock.calls[0][0][0].done).toBe(true);
  });
});

describe("Renaming", () => {
  it("renames a task and a subtask", async () => {
    setStore(({ state }) => {
      state.tasks = [
        {
          id: 1,
          title: "Old task",
          priority: "high",
          done: false,
          done_on: null,
          allocated_min: 60,
          subtasks: [
            { id: 1, title: "Old step", done: false, allocated_min: 0 },
          ],
          focus_area: null,
          carried_from_date: null,
        },
      ];
    });
    render(<Tasks />);
    fireEvent.click(screen.getByRole("button", { name: "Rename Old task" }));
    const field = screen.getByLabelText("New name for Old task");
    fireEvent.change(field, { target: { value: "  New task  " } });
    fireEvent.keyDown(field, { key: "Enter" });
    await waitFor(() => expect(api.updateTasks).toHaveBeenCalled());
    expect(savedTasks()[0].title).toBe("New task");

    cleanup();
    vi.mocked(api.updateTasks).mockClear();
    setStore(({ state }) => {
      state.tasks = [
        {
          id: 1,
          title: "Old task",
          priority: "high",
          done: false,
          done_on: null,
          allocated_min: 60,
          subtasks: [
            { id: 1, title: "Old step", done: false, allocated_min: 0 },
          ],
          focus_area: null,
          carried_from_date: null,
        },
      ];
    });
    render(<Tasks />);
    fireEvent.click(screen.getByRole("button", { name: /Show subtasks/ }));
    fireEvent.click(screen.getByRole("button", { name: "Rename Old step" }));
    const step = screen.getByLabelText("New name for Old step");
    fireEvent.change(step, { target: { value: "New step" } });
    fireEvent.blur(step);
    await waitFor(() => expect(api.updateTasks).toHaveBeenCalled());
    expect(savedTasks()[0].subtasks[0].title).toBe("New step");
  });

  it("keeps the name when the field is cleared or escaped", () => {
    setStore(({ state }) => {
      state.tasks = [
        {
          id: 1,
          title: "Keep",
          priority: "low",
          done: false,
          done_on: null,
          allocated_min: 60,
          subtasks: [],
          focus_area: null,
          carried_from_date: null,
        },
      ];
    });
    render(<Tasks />);
    fireEvent.click(screen.getByRole("button", { name: "Rename Keep" }));
    const field = screen.getByLabelText("New name for Keep");
    fireEvent.change(field, { target: { value: "   " } });
    fireEvent.blur(field);
    expect(api.updateTasks).not.toHaveBeenCalled();
    expect(screen.getByText("Keep")).toBeInTheDocument();
  });
});

describe("Limits", () => {
  function minutes(domain: string) {
    return screen.getByLabelText(`Daily minutes for ${domain}`);
  }

  it("shows the time left for each site", () => {
    setStore(({ state }) => {
      state.settings.allowance_sites.push({
        domain: "x.com",
        daily_min: 1,
      });
      state.today.site_used_sec["x.com"] = 60;
    });
    render(<Limits />);
    const row = (domain: string) => screen.getByText(domain).closest("li")!;
    expect(row("youtube.com")).toHaveTextContent("28mleft");
    expect(row("youtube.com")).toHaveTextContent("2m used");
    expect(row("reddit.com")).toHaveTextContent("10mleft");
    expect(row("x.com")).toHaveTextContent("Closed");
    expect(screen.getByLabelText("Time left on youtube.com")).toHaveAttribute(
      "value",
      "1680",
    );
  });

  it("can remove when not locked", async () => {
    setStore();
    render(<Limits />);
    fireEvent.click(screen.getByRole("button", { name: "Remove reddit.com" }));
    await waitFor(() =>
      expect(savedSettings().allowance_sites).toEqual([
        { domain: "youtube.com", daily_min: 30 },
      ]),
    );
  });

  it("saves one site's minutes when the field is left", async () => {
    setStore();
    render(<Limits />);
    const field = minutes("reddit.com");
    expect(field).toHaveValue(10);
    fireEvent.change(field, { target: { value: "25" } });
    expect(api.updateSettings).not.toHaveBeenCalled();
    fireEvent.blur(field);
    await waitFor(() =>
      expect(savedSettings().allowance_sites).toEqual([
        { domain: "youtube.com", daily_min: 30 },
        { domain: "reddit.com", daily_min: 25 },
      ]),
    );
  });

  it("doesn't save an empty or unchanged field", () => {
    setStore();
    render(<Limits />);
    const field = minutes("reddit.com");
    fireEvent.blur(field);
    fireEvent.change(field, { target: { value: "" } });
    fireEvent.blur(field);
    expect(api.updateSettings).not.toHaveBeenCalled();
    expect(field).toHaveValue(10);
  });

  it("caps minutes at the current value when locked, otherwise at 180", () => {
    setStore();
    const { unmount } = render(<Limits />);
    expect(minutes("reddit.com")).toHaveAttribute("max", "180");
    unmount();

    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
    });
    render(<Limits />);
    expect(minutes("reddit.com")).toHaveAttribute("max", "10");
    expect(minutes("youtube.com")).toHaveAttribute("max", "30");
  });

  it("shows the service's refusal", async () => {
    setStore();
    vi.mocked(api.updateSettings).mockRejectedValue({
      code: "LOCKED_LOOSENING",
      message: "The daily limit for reddit.com can't go up while locked.",
    });
    render(<Limits />);
    fireEvent.change(minutes("reddit.com"), { target: { value: "60" } });
    fireEvent.blur(minutes("reddit.com"));
    expect(await screen.findByRole("alert")).toHaveTextContent("can't go up");
  });

  it("disables remove and the Shorts toggle when locked, with a tooltip", () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
    });
    render(<Limits />);
    const remove = screen.getByRole("button", { name: "Remove reddit.com" });
    expect(remove).toBeDisabled();
    expect(remove.parentElement?.getAttribute("data-tip")).toMatch(
      /^Locked until /,
    );
    expect(screen.getByLabelText("Block Shorts and Reels")).toBeDisabled();
  });

  it("still allows adding and moving to always blocked when locked", async () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
    });
    render(<Limits />);
    fireEvent.change(screen.getByLabelText("Site to add"), {
      target: { value: "https://www.Twitch.tv/x" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() =>
      expect(savedSettings().allowance_sites).toContainEqual({
        domain: "twitch.tv",
        daily_min: 10,
      }),
    );

    vi.mocked(api.updateSettings).mockClear();
    fireEvent.click(screen.getAllByRole("button", { name: /Always block/ })[0]);
    await waitFor(() => {
      expect(savedSettings().always_blocked).toContain("youtube.com");
      expect(
        savedSettings().allowance_sites.map((site) => site.domain),
      ).not.toContain("youtube.com");
    });
  });

  it("switches lists and skips duplicates on both", async () => {
    setStore();
    render(<Limits />);
    fireEvent.change(screen.getByLabelText("Site to add"), {
      target: { value: "reddit.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "already on this list",
    );

    fireEvent.click(screen.getByRole("tab", { name: "Always blocked" }));
    expect(screen.getByText("tiktok.com")).toBeInTheDocument();
    expect(screen.queryByText("reddit.com")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Site to add"), {
      target: { value: "tiktok.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "tiktok.com is already",
    );
    expect(api.updateSettings).not.toHaveBeenCalled();
  });
});

describe("Settings", () => {
  it("saves edits", async () => {
    setStore();
    render(<FocusSettings />);
    fireEvent.change(screen.getByLabelText("Focus minutes"), {
      target: { value: "25" },
    });
    fireEvent.change(screen.getByLabelText("Daily focus target (hours)"), {
      target: { value: "8" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(api.updateSettings).toHaveBeenCalled());
    const saved = savedSettings();
    expect(saved.pomodoro_focus_min).toBe(25);
    expect(saved.daily_focus_target_min).toBe(480);
    expect(await screen.findByText("Saved.")).toBeInTheDocument();
  });

  it("doesn't overwrite limits changed elsewhere", async () => {
    setStore();
    const { rerender } = render(<FocusSettings />);
    setStore(({ state }) => {
      state.settings.allowance_sites = [];
    });
    rerender(<FocusSettings />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(savedSettings().allowance_sites).toEqual([]));
  });
});

describe("Stats", () => {
  beforeEach(() => {
    vi.mocked(api.getStats).mockResolvedValue({
      days: [],
      focus_totals_sec: { React: 7200 },
      blocked_attempts: 3,
    });
  });

  it("renders 7 bars and totals, then 30 bars", async () => {
    setStore();
    render(<StatsScreen />);
    expect(await screen.findAllByTestId("bar")).toHaveLength(7);
    expect(screen.getByText("2h")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(api.getStats).toHaveBeenCalledWith(7);

    fireEvent.click(screen.getByRole("tab", { name: "30 days" }));
    await waitFor(() => expect(api.getStats).toHaveBeenCalledWith(30));
    await waitFor(() => expect(screen.getAllByTestId("bar")).toHaveLength(30));
  });
});

describe("Lock", () => {
  function toPhraseStep() {
    render(<LockScreen />);
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    return screen.getByRole("button", { name: "Start the lock" });
  }

  it("keeps confirm disabled until the phrase matches exactly", () => {
    setStore();
    const confirm = toPhraseStep();
    const input = screen.getByLabelText("Commitment phrase");
    expect(confirm).toBeDisabled();
    for (const wrong of [
      "i commit to monk mode",
      "I commit to monk mode ",
      "I commit",
    ]) {
      fireEvent.change(input, { target: { value: wrong } });
      expect(confirm).toBeDisabled();
    }
    fireEvent.change(input, { target: { value: LOCK_PHRASE } });
    expect(confirm).toBeEnabled();
  });

  it("starts a 3-month lock by default", async () => {
    setStore();
    const confirm = toPhraseStep();
    fireEvent.change(screen.getByLabelText("Commitment phrase"), {
      target: { value: LOCK_PHRASE },
    });
    fireEvent.click(confirm);
    await waitFor(() => expect(api.startLock).toHaveBeenCalled());
    const days =
      (new Date(vi.mocked(api.startLock).mock.calls[0][0]).getTime() -
        Date.now()) /
      86_400_000;
    expect(days).toBeGreaterThan(88);
    expect(days).toBeLessThan(93);
  });

  it("needs a future date for a custom lock", () => {
    setStore();
    render(<LockScreen />);
    fireEvent.click(screen.getByLabelText("Custom date"));
    const next = screen.getByRole("button", { name: "Next" });
    expect(next).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Lock end date"), {
      target: { value: "2020-01-01" },
    });
    expect(next).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Lock end date"), {
      target: { value: "2099-01-01" },
    });
    expect(next).toBeEnabled();
  });

  it("shows status instead of the wizard while locked", () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
      state.lock_day = { day: 3, total: 90 };
    });
    render(<LockScreen />);
    expect(screen.getByText("Locked")).toBeInTheDocument();
    expect(screen.getByText("Day 3 of 90. Keep going.")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});

describe("Setup", () => {
  it("walks through, waits for the extension, and marks setup done", async () => {
    const snapshot = setStore(({ state }) => {
      state.settings.setup_done = false;
    });
    const { rerender } = render(<Setup />);
    fireEvent.click(screen.getByRole("button", { name: "Get started" }));

    expect(screen.getByText("The service is running.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));

    expect(screen.getByText("Waiting for extension…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
    setStore((s) => {
      s.state.settings.setup_done = false;
      s.connected_browsers = ["chrome.exe"];
    });
    rerender(<Setup />);
    expect(screen.getByText("Connected in Chrome.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));

    expect(screen.getByText(snapshot.state.settings.always_blocked[0]));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));

    fireEvent.click(screen.getByRole("button", { name: "Skip for now" }));
    await waitFor(() => expect(savedSettings().setup_done).toBe(true));
    expect(api.startLock).not.toHaveBeenCalled();
  });
});

it("About states the limitations", () => {
  render(
    <MemoryRouter>
      <About />
    </MemoryRouter>,
  );
  expect(screen.getByText(/slow and annoying/)).toBeInTheDocument();
  expect(screen.getByText(/Safe Mode/)).toBeInTheDocument();
});
