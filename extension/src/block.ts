import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/source-serif-4/latin-500.css";
import "./block.css";
import logo from "../../app/src/assets/logo.svg?raw";
import { STATE_KEY, formatDuration, streakText } from "./view";
import type { State } from "./types/State";

const logoElement = document.getElementById("logo");
if (logoElement) logoElement.innerHTML = logo;

function row(name: string, focusSec: number, targetMin: number): HTMLElement {
  const wrapper = document.createElement("div");

  const label = document.createElement("div");
  label.className = "mb-1 flex justify-between text-sm";
  const nameElement = document.createElement("span");
  nameElement.textContent = name;
  const amount = document.createElement("span");
  amount.className = "text-neutral";
  amount.textContent = `${formatDuration(focusSec)} / ${formatDuration(targetMin * 60)}`;
  label.append(nameElement, amount);

  const bar = document.createElement("progress");
  bar.className = "progress progress-secondary w-full";
  bar.max = Math.max(targetMin * 60, 1);
  bar.value = focusSec;

  wrapper.append(label, bar);
  return wrapper;
}

function render(state: State | undefined) {
  const streak = document.getElementById("streak");
  const progress = document.getElementById("progress");
  if (!state || !streak || !progress) return;

  streak.textContent = streakText(state.streak_days, state.lock_day);
  progress.replaceChildren(
    ...state.settings.study_blocks.map((block) =>
      row(block.name, state.today.focus_sec[block.name] ?? 0, block.target_min),
    ),
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
