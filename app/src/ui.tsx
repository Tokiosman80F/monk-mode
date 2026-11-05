import type { ReactNode } from "react";
import logo from "./assets/logo.svg?raw";
import { formatLeft } from "./lib";
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
      className={`inline-block text-primary ${className} ${breathing ? "animate-breathe" : ""}`}
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
              <h2 className="font-display text-lg font-semibold">{title}</h2>
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
    <h1 className="font-display text-3xl font-semibold tracking-tight">
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

const RING_RADIUS = 26;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;
const RING_WARN_SEC = 120;

/** Time left on one site's limit. The ring empties as the time goes. */
export function LimitRing({
  domain,
  leftSec,
  limitSec,
  closed = false,
}: {
  domain: string;
  leftSec: number;
  limitSec: number;
  closed?: boolean;
}) {
  const share = limitSec > 0 ? leftSec / limitSec : 0;
  const text = closed ? "Closed" : formatLeft(leftSec);
  const tone = leftSec <= RING_WARN_SEC ? "stroke-error" : "stroke-primary";
  const name = domain.replace(/\.com$/, "");
  return (
    <div
      role="img"
      aria-label={`${domain}: ${text}`}
      className="flex min-w-0 flex-col items-center gap-1"
    >
      <div className="relative h-[68px] w-[68px]">
        <svg
          viewBox="0 0 64 64"
          className="h-full w-full -rotate-90 overflow-visible"
        >
          <circle
            cx={32}
            cy={32}
            r={RING_RADIUS}
            fill="none"
            strokeWidth={5}
            className="stroke-base-300"
          />
          <circle
            cx={32}
            cy={32}
            r={RING_RADIUS}
            fill="none"
            strokeWidth={5}
            strokeLinecap="round"
            strokeDasharray={RING_LENGTH}
            strokeDashoffset={RING_LENGTH * (1 - (closed ? 0 : share))}
            className={`${tone} drop-shadow-glow transition-[stroke-dashoffset] duration-500`}
          />
        </svg>
        <span
          className={`absolute inset-0 flex items-center justify-center font-display font-semibold ${
            text === "Closed" ? "text-[11px] text-neutral" : "text-sm"
          }`}
        >
          {text}
        </span>
      </div>
      <span className="w-full truncate text-center text-xs text-neutral">
        {name}
      </span>
    </div>
  );
}

export function LevelBar({ progress }: { progress: Progress }) {
  const { level, level_xp: into, next_level_xp: next } = progress;
  return (
    <div className="flex w-56 items-center gap-3">
      <span
        key={level}
        className="flex h-10 w-10 shrink-0 animate-fade-up items-center justify-center rounded-full bg-primary font-display text-base font-bold text-primary-content shadow-glow"
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
          className="progress progress-primary h-2 w-full"
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
