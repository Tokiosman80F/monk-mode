import type { ReactNode } from "react";
import logo from "./assets/logo.svg?raw";

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
  children,
}: {
  title?: string;
  children: ReactNode;
}) {
  return (
    <section className="card animate-fade-up border border-base-300 bg-base-200 shadow-sm">
      <div className="card-body gap-4 p-6">
        {title && <h2 className="font-serif text-xl font-medium">{title}</h2>}
        {children}
      </div>
    </section>
  );
}

export function PageTitle({ children }: { children: ReactNode }) {
  return <h1 className="font-serif text-3xl font-medium">{children}</h1>;
}

export function ErrorNote({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <p role="alert" className="alert alert-error text-sm">
      {message}
    </p>
  );
}

export function ProgressRow({
  label,
  detail,
  value,
  max,
  tone = "progress-secondary",
}: {
  label: string;
  detail: string;
  value: number;
  max: number;
  tone?: string;
}) {
  return (
    <div>
      <div className="mb-1 flex justify-between text-sm">
        <span>{label}</span>
        <span className="text-neutral">{detail}</span>
      </div>
      <progress
        className={`progress w-full ${tone}`}
        aria-label={label}
        value={value}
        max={Math.max(max, 1)}
      />
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
