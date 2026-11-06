import { X } from "lucide-react";
import { api } from "../api";
import { timeLeftText } from "../lib";
import { useStore } from "../store";

/** The small always-on-top timer window: only the time left. */
export default function Mini() {
  const pomodoro = useStore((s) => s.snapshot?.state.pomodoro);
  const running = pomodoro && pomodoro.phase !== "idle";
  const focusing = pomodoro?.phase === "focus";

  return (
    <main
      data-tauri-drag-region
      className="group flex h-screen cursor-default select-none items-center gap-2 border border-base-300 bg-base-200 px-3 text-base-content"
    >
      <span
        aria-hidden="true"
        title={focusing ? "Focus" : "Break"}
        className={`h-2 w-2 shrink-0 rounded-full ${focusing ? "animate-breathe bg-secondary" : "bg-accent"}`}
      />
      <p
        data-tauri-drag-region
        role="timer"
        aria-label={focusing ? "Focus time left" : "Break time left"}
        className="flex-1 whitespace-nowrap text-sm font-medium tabular-nums"
      >
        {running ? timeLeftText(pomodoro.remaining_sec) : "Idle"}
      </p>
      <button
        className="opacity-0 transition-opacity duration-150 focus:opacity-100 group-hover:opacity-100"
        aria-label="Hide the timer"
        onClick={() => void api.dismissMini()}
      >
        <X size={12} aria-hidden="true" className="text-neutral" />
      </button>
    </main>
  );
}
