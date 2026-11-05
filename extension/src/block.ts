import "@fontsource/inter-tight/latin-300.css";
import "@fontsource/inter-tight/latin-400.css";
import "@fontsource/inter-tight/latin-500.css";
import "./block.css";
import logo from "../../app/src/assets/logo.svg?raw";
import { STATE_KEY, formatDuration, streakText, topTasks } from "./view";
import type { State } from "./types/State";

const TASKS_SHOWN = 3;

const logoElement = document.getElementById("logo");
if (logoElement) logoElement.innerHTML = logo;

function focusRow(focusSec: number, targetMin: number): HTMLElement {
  const wrapper = document.createElement("div");

  const label = document.createElement("div");
  label.className = "mb-1 flex justify-between text-sm";
  const name = document.createElement("span");
  name.textContent = "Focus today";
  const amount = document.createElement("span");
  amount.className = "text-neutral";
  amount.textContent = `${formatDuration(focusSec)} / ${formatDuration(targetMin * 60)}`;
  label.append(name, amount);

  const bar = document.createElement("progress");
  bar.className = "progress progress-secondary w-full";
  bar.max = Math.max(targetMin * 60, 1);
  bar.value = focusSec;

  wrapper.append(label, bar);
  return wrapper;
}

function taskRow(title: string): HTMLElement {
  const row = document.createElement("p");
  row.className =
    "truncate rounded-btn border border-base-300 bg-base-200 px-3 py-2";
  row.textContent = title;
  return row;
}

function render(state: State | undefined) {
  const streak = document.getElementById("streak");
  const progress = document.getElementById("progress");
  if (!state || !streak || !progress) return;

  streak.textContent = streakText(state.streak_days, state.lock_day);
  const focusSec = Object.values(state.today.focus_sec).reduce<number>(
    (sum, sec) => sum + (sec ?? 0),
    0,
  );
  progress.replaceChildren(
    focusRow(focusSec, state.settings.daily_focus_target_min),
    ...topTasks(state.tasks, TASKS_SHOWN).map((task) => taskRow(task.title)),
  );
}

chrome.storage.session
  .get(STATE_KEY)
  .then((stored) => render(stored[STATE_KEY]));
chrome.storage.session.onChanged.addListener((changes) => {
  if (changes[STATE_KEY]) render(changes[STATE_KEY].newValue);
});

document.getElementById("open-plan")?.addEventListener("click", () => {
  void chrome.runtime.sendMessage({ type: "open_app" });
});

void chrome.runtime.sendMessage({ type: "blocked_attempt" });
