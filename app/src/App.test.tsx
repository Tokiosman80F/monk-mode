import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "./api";
import App from "./App";
import { useStore } from "./store";
import { makeSnapshot, setStore } from "./test/fixtures";

vi.mock("./api", () => ({
  errorText: (e: { message?: string }) => e.message ?? String(e),
  api: {
    getState: vi.fn(),
    getStats: vi.fn(async () => ({
      days: [],
      focus_totals_sec: {},
      blocked_attempts: 0,
    })),
    onStateChanged: vi.fn(async () => () => {}),
    onServiceStatus: vi.fn(async () => () => {}),
  },
}));

beforeEach(() => {
  useStore.setState({ snapshot: null, connected: null });
  window.location.hash = "";
  vi.mocked(api.getState).mockResolvedValue(makeSnapshot());
});

async function renderApp() {
  await act(async () => {
    render(<App />);
  });
}

describe("App", () => {
  it("loads state and navigates between all screens", async () => {
    await renderApp();
    expect(
      screen.getByRole("heading", { level: 1, name: "Today" }),
    ).toBeInTheDocument();

    for (const name of ["Block lists", "Schedule", "Stats", "Lock", "About"]) {
      await act(async () => {
        fireEvent.click(screen.getByRole("link", { name }));
      });
      expect(
        screen.getByRole("heading", { level: 1, name }),
      ).toBeInTheDocument();
    }
  });

  it("marks the current screen in the sidebar", async () => {
    await renderApp();
    expect(screen.getByRole("link", { name: "Today" })).toHaveClass("active");
    expect(screen.getByRole("link", { name: "Stats" })).not.toHaveClass(
      "active",
    );
  });

  it("shows a banner when the service isn't running", async () => {
    vi.mocked(api.getState).mockRejectedValue({ message: "down" });
    await renderApp();
    expect(screen.getByRole("alert")).toHaveTextContent("Service not running");
  });

  it("removes the banner when the service comes back", async () => {
    vi.mocked(api.getState).mockRejectedValue({ message: "down" });
    await renderApp();
    act(() => {
      setStore();
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows setup only when setup isn't done", async () => {
    const fresh = makeSnapshot();
    fresh.state.settings.setup_done = false;
    vi.mocked(api.getState).mockResolvedValue(fresh);
    await renderApp();
    expect(
      screen.getByRole("heading", { name: "Welcome" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Today" })).toBeNull();
  });

  it("doesn't show setup before the service has answered", async () => {
    vi.mocked(api.getState).mockRejectedValue({ message: "down" });
    await renderApp();
    expect(screen.queryByRole("heading", { name: "Welcome" })).toBeNull();
  });

  it("the logo breathes during focus", async () => {
    await renderApp();
    expect(screen.getByLabelText("Monk Mode logo")).not.toHaveClass(
      "animate-breathe",
    );
    act(() => {
      setStore(({ state }) => {
        state.pomodoro = {
          phase: "focus",
          block_name: "React",
          started_at: "2026-11-05T09:00:00Z",
          remaining_sec: 60,
        };
      });
    });
    expect(screen.getAllByLabelText("Monk Mode logo")[0]).toHaveClass(
      "animate-breathe",
    );
  });
});

describe("store", () => {
  it("receives pushes through the listener", async () => {
    await renderApp();
    const handler = vi.mocked(api.onStateChanged).mock.calls.at(-1)![0];
    const pushed = makeSnapshot();
    pushed.state.streak_days = 99;
    act(() => handler(pushed));
    expect(useStore.getState().snapshot?.state.streak_days).toBe(99);
  });
});
