import { useEffect, useState, type ReactNode } from "react";
import logo from "./assets/logo.svg?raw";
import { quoteFor } from "./quotes";
import type { Progress } from "./types/Progress";

export function Logo({
  breathing = false,
  className = "w-8",
}: {
  breathing?: boolean;
  className?: string;
}) {
  return (
    <span
      className={`inline-block text-secondary ${className} ${breathing ? "animate-breathe" : ""}`}
      role="img"
      aria-label="Monk Mode logo"
      dangerouslySetInnerHTML={{ __html: logo }}
    />
  );
}

export function Card({
  title,
  action,
  children,
}: {
  title?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-box border border-base-300 bg-base-200">
      <div className="flex flex-col gap-4 p-5">
        {(title || action) && (
          <div className="flex items-center justify-between gap-4">
            {title && (
              <h2 className="font-display text-lg font-medium">{title}</h2>
            )}
            {action}
          </div>
        )}
        {children}
      </div>
    </section>
  );
}

export function PageTitle({ children }: { children: ReactNode }) {
  return (
    <h1 className="font-display text-3xl font-light tracking-tight">
      {children}
    </h1>
  );
}

export function ErrorNote({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <p role="alert" className="alert alert-error text-sm">
      {message}
    </p>
  );
}

/** Today's date and the time, with the day's quote beside them. */
export function DayHeader() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);
  const quote = quoteFor(now);

  return (
    <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-8 gap-y-2">
      <div>
        <p className="font-display text-5xl font-light tabular-nums leading-none tracking-tight">
          {now.toLocaleTimeString(undefined, {
            hour: "numeric",
            minute: "2-digit",
          })}
        </p>
        <h1 className="mt-2 text-neutral">
          {now.toLocaleDateString(undefined, {
            weekday: "long",
            day: "numeric",
            month: "long",
            year: "numeric",
          })}
        </h1>
      </div>
      <figure className="max-w-xs">
        <blockquote className="text-sm font-medium leading-snug">
          {quote.text}
        </blockquote>
        <figcaption className="mt-1 text-xs text-neutral">
          {quote.by}
        </figcaption>
      </figure>
    </div>
  );
}

export function LevelBar({ progress }: { progress: Progress }) {
  const { level, level_xp: into, next_level_xp: next } = progress;
  return (
    <div className="flex w-52 items-center gap-3">
      <span
        key={level}
        className="flex h-10 w-10 shrink-0 animate-fade-in items-center justify-center rounded-full bg-accent font-display text-base font-medium text-accent-content"
        aria-label={`Level ${level}`}
      >
        {level}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex justify-between text-xs text-neutral">
          <span>Level {level}</span>
          <span>
            {into} / {next} XP
          </span>
        </div>
        <progress
          className="progress progress-accent h-1.5 w-full"
          aria-label="XP to the next level"
          value={into}
          max={Math.max(next, 1)}
        />
      </div>
    </div>
  );
}

/** A disabled control's explanation. Only wraps when there's something to say. */
export function LockTip({
  tip,
  children,
}: {
  tip: string;
  children: ReactNode;
}) {
  if (!tip) return <>{children}</>;
  return (
    <span className="tooltip tooltip-left" data-tip={tip}>
      {children}
    </span>
  );
}
