import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../api";
import { LOCK_PHRASE } from "../lib";
import { FUTURE_LOCK, makeSnapshot, setStore } from "../test/fixtures";
import About from "./About";
import BlockLists from "./BlockLists";
import LockScreen from "./Lock";
import Pomodoro from "./Pomodoro";
import Schedule from "./Schedule";
import Setup from "./Setup";
import StatsScreen from "./Stats";
import Today from "./Today";

vi.mock("../api", () => ({
  errorText: (e: { message?: string }) => e.message ?? String(e),
  api: {
    getState: vi.fn(),
    updateSettings: vi.fn(),
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
  vi.mocked(api.startPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.stopPomodoro).mockResolvedValue(makeSnapshot());
  vi.mocked(api.startLock).mockResolvedValue(makeSnapshot());
});

function savedSettings() {
  return vi.mocked(api.updateSettings).mock.calls[0][0];
}

describe("Today", () => {
  it("shows allowance, progress, streak and the start button", () => {
    setStore();
    render(<Today />);
    expect(screen.getByText("8:00")).toBeInTheDocument();
    expect(screen.getByText("8 minutes left for today.")).toBeInTheDocument();
    expect(screen.getByText("1h 40m / 3h")).toBeInTheDocument();
    expect(screen.getByLabelText("React")).toHaveAttribute("value", "6000");
    expect(screen.getByText("4 days")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Start Pomodoro/ }),
    ).toBeInTheDocument();
  });

  it("shows the day counter while locked", () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
      state.lock_day = { day: 12, total: 90 };
    });
    render(<Today />);
    expect(screen.getByText("Day 12 of 90. Keep going.")).toBeInTheDocument();
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

describe("Pomodoro", () => {
  it("starts on the chosen block", async () => {
    setStore();
    render(<Pomodoro />);
    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: "Building" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Start Pomodoro/ }));
    await waitFor(() =>
      expect(api.startPomodoro).toHaveBeenCalledWith("Building"),
    );
  });

  it("counts down from what the service pushes, and can stop", async () => {
    setStore(({ state }) => {
      state.pomodoro = {
        phase: "focus",
        block_name: "React",
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
        block_name: "React",
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
    fireEvent.click(screen.getByRole("button", { name: /Start Pomodoro/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Nope.");
  });
});

describe("Block lists", () => {
  it("can remove when not locked", async () => {
    setStore();
    render(<BlockLists />);
    fireEvent.click(screen.getByRole("button", { name: "Remove reddit.com" }));
    await waitFor(() =>
      expect(savedSettings().allowance_sites).toEqual(["youtube.com"]),
    );
  });

  it("disables remove and the Shorts toggle when locked, with a tooltip", () => {
    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
    });
    render(<BlockLists />);
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
    render(<BlockLists />);
    fireEvent.change(screen.getByLabelText("Site to add"), {
      target: { value: "https://www.Twitch.tv/x" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() =>
      expect(savedSettings().allowance_sites).toContain("twitch.tv"),
    );

    vi.mocked(api.updateSettings).mockClear();
    fireEvent.click(
      screen.getAllByRole("button", { name: /Move to always blocked/ })[0],
    );
    await waitFor(() => {
      expect(savedSettings().always_blocked).toContain("youtube.com");
      expect(savedSettings().allowance_sites).not.toContain("youtube.com");
    });
  });

  it("switches lists and skips duplicates", async () => {
    setStore();
    render(<BlockLists />);
    fireEvent.click(screen.getByRole("tab", { name: "Always blocked" }));
    expect(screen.getByText("tiktok.com")).toBeInTheDocument();
    expect(screen.queryByText("reddit.com")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Site to add"), {
      target: { value: "tiktok.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "already on this list",
    );
    expect(api.updateSettings).not.toHaveBeenCalled();
  });
});

describe("Schedule", () => {
  it("saves edits", async () => {
    setStore();
    render(<Schedule />);
    fireEvent.change(screen.getByLabelText("Minutes per day"), {
      target: { value: "5" },
    });
    fireEvent.change(screen.getByLabelText("Focus minutes"), {
      target: { value: "25" },
    });
    fireEvent.change(screen.getAllByLabelText("Name")[0], {
      target: { value: "React docs" },
    });
    fireEvent.change(screen.getAllByLabelText("Hours")[0], {
      target: { value: "2.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(api.updateSettings).toHaveBeenCalled());
    const saved = savedSettings();
    expect(saved.daily_allowance_min).toBe(5);
    expect(saved.pomodoro_focus_min).toBe(25);
    expect(saved.study_blocks[0]).toEqual({
      name: "React docs",
      target_min: 150,
    });
    expect(await screen.findByText("Saved.")).toBeInTheDocument();
  });

  it("caps the allowance at the current value when locked, otherwise at 60", () => {
    setStore();
    const { unmount } = render(<Schedule />);
    expect(screen.getByLabelText("Minutes per day")).toHaveAttribute(
      "max",
      "60",
    );
    unmount();

    setStore(({ state }) => {
      state.lock = FUTURE_LOCK;
    });
    render(<Schedule />);
    expect(screen.getByLabelText("Minutes per day")).toHaveAttribute(
      "max",
      "10",
    );
  });

  it("shows the service's refusal", async () => {
    setStore();
    vi.mocked(api.updateSettings).mockRejectedValue({
      code: "LOCKED_LOOSENING",
      message: "The daily allowance can't go up while locked.",
    });
    render(<Schedule />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("can't go up");
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
