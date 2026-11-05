import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../api";
import { LOCK_PHRASE } from "../lib";
import { FUTURE_LOCK, makeSnapshot, setStore } from "../test/fixtures";
import About from "./About";
import FocusSettings from "./FocusSettings";
import Limits from "./Limits";
import LockScreen from "./Lock";
import Pomodoro from "./Pomodoro";
import Rewards from "./Rewards";
import Setup from "./Setup";
import StatsScreen from "./Stats";
import Tasks from "./Tasks";
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
    getStats: vi.fn(),
    installService: vi.fn(),
    openExtensionFolder: vi.fn(),
    onStateChanged: vi.fn(async () => () => {}),
    onServiceStatus: vi.fn(async () => () => {}),
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.updateSettings).mockResolvedValue(makeSnapshot());
  vi.mocked(api.updateTasks).mockResolvedValue(makeSnapshot());
  vi.mocked(api.startPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.stopPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.startLock).mockResolvedValue(makeSnapshot());
});

function savedSettings() {
  return vi.mocked(api.updateSettings).mock.calls[0][0];
}

function savedTasks() {
  return vi.mocked(api.updateTasks).mock.calls[0][0];
}

function fillComposer(title: string, time: string) {
  fireEvent.change(screen.getByLabelText("New task"), {
    target: { value: title },
  });
  fireEvent.change(screen.getByLabelText("Time for the new task"), {
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
  it("shows all four quadrants with tasks in the right one", () => {
    setStore();
    render(<Tasks />);
    const doFirst = screen.getByRole("region", { name: "Do first" });
    expect(within(doFirst).getByText("Ship the page")).toBeInTheDocument();
    expect(
      within(doFirst).getByRole("button", {
        name: /^Time for Ship the page: 1h 30m/,
      }),
    ).toBeInTheDocument();
    const schedule = screen.getByRole("region", { name: "Schedule" });
    expect(within(schedule).getByText("Learn hooks")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Delegate" })).toBeVisible();
    expect(screen.getByRole("region", { name: "Eliminate" })).toBeVisible();
  });

  it("adds a task with its time, and the quadrant sets the priority", async () => {
    setStore();
    render(<Tasks />);
    fillComposer("  Read docs  ", "3h");
    fireEvent.click(screen.getByRole("button", { name: "Urgent" }));
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() =>
      expect(savedTasks()[2]).toMatchObject({
        id: 3,
        title: "Read docs",
        allocated_min: 180,
        quadrant: "do_first",
        priority: "high",
      }),
    );
  });

  it("each urgent and important answer picks its quadrant", async () => {
    setStore();
    render(<Tasks />);
    fillComposer("Reply", "10m");
    fireEvent.click(screen.getByRole("button", { name: "Urgent" }));
    fireEvent.click(screen.getByRole("button", { name: "Important" }));
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() =>
      expect(savedTasks()[2]).toMatchObject({
        quadrant: "delegate",
        priority: "low",
      }),
    );
  });

  it("lets the priority be changed from what the quadrant suggests", async () => {
    setStore();
    render(<Tasks />);
    fillComposer("Read", "1h");
    fireEvent.click(screen.getByLabelText(/^New task priority: Medium/));
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(savedTasks()[2].priority).toBe("low"));
  });

  it("needs a time for a new task", () => {
    setStore();
    render(<Tasks />);
    fillComposer("Read", "");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(screen.getByRole("alert")).toHaveTextContent("How long");
    fillComposer("Read", "soon");
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

  it("moves a task by dragging it to another quadrant", async () => {
    setStore();
    render(<Tasks />);
    const data = { getData: () => "1" };
    fireEvent.drop(screen.getByRole("region", { name: "Delegate" }), {
      dataTransfer: data,
    });
    await waitFor(() => expect(savedTasks()[0].quadrant).toBe("delegate"));
  });

  it("moves a task with buttons too", async () => {
    setStore();
    render(<Tasks />);
    fireEvent.click(
      screen.getByRole("button", { name: "Show subtasks of Learn hooks" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Eliminate" }));
    await waitFor(() => expect(savedTasks()[0].quadrant).toBe("eliminate"));
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

  it("counts down from what the service pushes, and can stop", async () => {
    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "React",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 125,
      };
    });
    const { rerender } = render(<Pomodoro />);
    expect(screen.getByRole("timer")).toHaveTextContent("2:05");
    expect(screen.getByText("Focus: React")).toBeInTheDocument();
    expect(screen.getByLabelText("Monk Mode logo")).toHaveClass(
      "animate-breathe",
    );

    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        task_title: "React",
        started_at: "2026-11-05T09:00:00Z",
        remaining_sec: 124,
      };
    });
    rerender(<Pomodoro />);
    expect(screen.getByRole("timer")).toHaveTextContent("2:04");

    fireEvent.click(screen.getByRole("button", { name: /Stop/ }));
    await waitFor(() => expect(api.stopPomodoro).toHaveBeenCalled());
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

describe("Limits", () => {
  function minutes(domain: string) {
    return screen.getByLabelText(`Daily minutes for ${domain}`);
  }

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
