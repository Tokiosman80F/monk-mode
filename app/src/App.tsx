import {
  BarChart3,
  CalendarClock,
  Info,
  Lock as LockIcon,
  ShieldBan,
  Sun,
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
import BlockLists from "./screens/BlockLists";
import LockScreen from "./screens/Lock";
import Schedule from "./screens/Schedule";
import Setup from "./screens/Setup";
import StatsScreen from "./screens/Stats";
import Today from "./screens/Today";
import { useServiceState, useStore } from "./store";
import { Logo } from "./ui";

const NAV: [string, string, LucideIcon][] = [
  ["/", "Today", Sun],
  ["/lists", "Block lists", ShieldBan],
  ["/schedule", "Schedule", CalendarClock],
  ["/stats", "Stats", BarChart3],
  ["/lock", "Lock", LockIcon],
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
    <li>
      <NavLink
        to={to}
        end
        className={({ isActive }) =>
          isActive ? "active !bg-primary !text-primary-content" : ""
        }
      >
        <Icon size={18} />
        {label}
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
      <aside className="flex w-56 shrink-0 flex-col gap-6 border-r border-base-300 bg-base-200 p-4">
        <div className="flex items-center gap-2 px-2 pt-2">
          <Logo breathing={focusing} />
          <span className="font-serif text-xl font-medium">Monk Mode</span>
        </div>
        <ul className="menu gap-1 p-0">
          {NAV.map(([to, label, Icon]) => (
            <NavItem key={to} to={to} label={label} Icon={Icon} />
          ))}
        </ul>
        <ul className="menu mt-auto p-0">
          <NavItem to="/about" label="About" Icon={Info} />
        </ul>
      </aside>
      <main className="flex-1 overflow-y-auto">
        <div
          key={pathname}
          className="mx-auto flex max-w-[760px] animate-fade-up flex-col gap-6 p-6"
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
          <Route path="/lists" element={<BlockLists />} />
          <Route path="/schedule" element={<Schedule />} />
          <Route path="/stats" element={<StatsScreen />} />
          <Route path="/lock" element={<LockScreen />} />
          <Route path="/about" element={<About />} />
          <Route path="*" element={<Navigate to="/" />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
