import {
  BarChart3,
  Gauge,
  Info,
  LayoutDashboard,
  Lock as LockIcon,
  Settings as SettingsIcon,
  Trophy,
  type LucideIcon,
} from "lucide-react";
import {
  HashRouter,
  NavLink,
  Navigate,
  Outlet,
  Route,
  Routes,
  useLocation,
} from "react-router-dom";
import About from "./screens/About";
import FocusSettings from "./screens/FocusSettings";
import Limits from "./screens/Limits";
import LockScreen from "./screens/Lock";
import Rewards from "./screens/Rewards";
import Setup from "./screens/Setup";
import StatsScreen from "./screens/Stats";
import Today from "./screens/Today";
import { useServiceState, useStore } from "./store";
import { Logo } from "./ui";

const NAV: [string, string, LucideIcon][] = [
  ["/", "Today", LayoutDashboard],
  ["/limits", "Limits", Gauge],
  ["/rewards", "Rewards", Trophy],
  ["/stats", "Stats", BarChart3],
  ["/lock", "Lock", LockIcon],
];

const NAV_BOTTOM: [string, string, LucideIcon][] = [
  ["/settings", "Settings", SettingsIcon],
  ["/about", "About", Info],
];

function NavItem({
  to,
  label,
  Icon,
}: {
  to: string;
  label: string;
  Icon: LucideIcon;
}) {
  return (
    <li className="tooltip tooltip-right" data-tip={label}>
      <NavLink
        to={to}
        end
        aria-label={label}
        className={({ isActive }) =>
          `flex h-11 w-11 items-center justify-center rounded-btn transition-colors duration-150 ${
            isActive
              ? "active bg-primary text-primary-content shadow-glow"
              : "text-neutral hover:bg-base-300 hover:text-base-content"
          }`
        }
      >
        <Icon size={20} aria-hidden="true" />
      </NavLink>
    </li>
  );
}

function Layout() {
  const { pathname } = useLocation();
  const connected = useStore((s) => s.connected);
  const focusing = useStore(
    (s) => s.snapshot?.state.pomodoro.phase === "focus",
  );

  return (
    <div className="flex h-screen bg-base-100 text-base-content">
      <aside className="z-10 flex w-[72px] shrink-0 flex-col items-center gap-6 border-r border-base-300 bg-base-200 py-4">
        <Logo breathing={focusing} className="w-9" />
        <ul className="flex flex-col gap-2">
          {NAV.map(([to, label, Icon]) => (
            <NavItem key={to} to={to} label={label} Icon={Icon} />
          ))}
        </ul>
        <ul className="mt-auto flex flex-col gap-2">
          {NAV_BOTTOM.map(([to, label, Icon]) => (
            <NavItem key={to} to={to} label={label} Icon={Icon} />
          ))}
        </ul>
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto">
        <div
          key={pathname}
          className="mx-auto flex max-w-[1080px] animate-fade-in flex-col gap-5 p-6"
        >
          {connected === false && (
            <p role="alert" className="alert alert-warning">
              Service not running. Nothing is being blocked until it&apos;s
              back.
            </p>
          )}
          <Outlet />
        </div>
      </main>
    </div>
  );
}

export default function App() {
  useServiceState();
  const setupDone = useStore((s) => s.snapshot?.state.settings.setup_done);

  // Only a service that has answered can say setup isn't done.
  if (setupDone === false) return <Setup />;

  return (
    <HashRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Today />} />
          <Route path="/limits" element={<Limits />} />
          <Route path="/rewards" element={<Rewards />} />
          <Route path="/stats" element={<StatsScreen />} />
          <Route path="/lock" element={<LockScreen />} />
          <Route path="/settings" element={<FocusSettings />} />
          <Route path="/about" element={<About />} />
          <Route path="*" element={<Navigate to="/" />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
