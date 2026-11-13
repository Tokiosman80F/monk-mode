import {
  Award,
  CheckCheck,
  Clock,
  Flame,
  ListChecks,
  Lock as LockIcon,
  Mountain,
  Sprout,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { useStore } from "../store";
import type { Badge } from "../types/Badge";
import { Card, PageTitle } from "../ui";

const BADGES: [Badge, string, string, LucideIcon][] = [
  ["first_focus", "First session", "Focus for the first time", Sprout],
  ["focus10h", "Ten hours", "Reach 10 hours of focus", Clock],
  ["focus100h", "Hundred hours", "Reach 100 hours of focus", Zap],
  ["streak3", "Three in a row", "Hit your target 3 days running", Flame],
  ["streak7", "Full week", "Hit your target 7 days running", Flame],
  ["streak30", "Full month", "Hit your target 30 days running", Award],
  ["streak90", "Monk", "Hit your target 90 days running", Mountain],
  ["tasks10", "Getting it done", "Finish 10 tasks", ListChecks],
  ["tasks100", "Finisher", "Finish 100 tasks", CheckCheck],
];

const EARNING: [string, string][] = [
  ["Each minute of focus", "1 XP"],
  ["High priority task", "50 XP"],
  ["Medium priority task", "30 XP"],
  ["Low priority task", "15 XP"],
  ["Subtask", "5 XP"],
];

export default function Rewards() {
  const progress = useStore((s) => s.snapshot?.state.progress);
  if (!progress) return <PageTitle>Rewards</PageTitle>;

  const { level, level_xp: into, next_level_xp: next, xp, badges } = progress;

  return (
    <>
      <PageTitle>Rewards</PageTitle>

      <Card>
        <div className="flex flex-wrap items-center gap-6">
          <span className="flex h-24 w-24 shrink-0 items-center justify-center rounded-full bg-accent font-display text-5xl font-light text-accent-content">
            {level}
          </span>
          <div className="min-w-48 flex-1">
            <h2 className="font-display text-2xl font-medium">Level {level}</h2>
            <p className="mb-2 text-sm text-neutral">
              {next - into} XP to level {level + 1}. {xp} XP in total.
            </p>
            <progress
              className="progress progress-accent w-full"
              aria-label="XP to the next level"
              value={into}
              max={Math.max(next, 1)}
            />
          </div>
        </div>
      </Card>

      <div className="grid items-start gap-5 md:grid-cols-[minmax(0,8fr)_minmax(0,5fr)]">
        <Card title={`Badges: ${badges.length} of ${BADGES.length}`}>
          <ul className="grid grid-cols-2 gap-3 lg:grid-cols-3">
            {BADGES.map(([id, name, how, Icon]) => {
              const earned = badges.includes(id);
              return (
                <li
                  key={id}
                  data-testid={earned ? "earned" : "unearned"}
                  className={`card card-bordered ${
                    earned ? "border-accent" : "border-base-300"
                  }`}
                >
                  <div className="card-body items-center gap-2 p-4 text-center">
                    {earned ? (
                      <Icon
                        size={28}
                        aria-hidden="true"
                        className="text-accent"
                      />
                    ) : (
                      <LockIcon
                        size={28}
                        aria-hidden="true"
                        className="text-base-300"
                      />
                    )}
                    <span className="font-medium">{name}</span>
                    <span className="text-xs text-neutral">{how}</span>
                  </div>
                </li>
              );
            })}
          </ul>
        </Card>

        <Card title="How to earn XP">
          <ul className="flex flex-col gap-2 text-sm">
            {EARNING.map(([what, xpText]) => (
              <li key={what} className="flex justify-between gap-4">
                <span>{what}</span>
                <span className="font-medium text-accent">{xpText}</span>
              </li>
            ))}
          </ul>
          <p className="text-xs text-neutral">
            Tasks can earn up to 200 XP a day. Focus time has no daily cap.
            Unticking a task takes its XP back, and you never lose XP for a
            slip.
          </p>
        </Card>
      </div>
    </>
  );
}
