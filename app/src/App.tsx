import { Hourglass } from "lucide-react";
import logo from "./assets/logo.svg?raw";

export default function App() {
  return (
    <main className="flex min-h-screen animate-fade-up flex-col items-center justify-center gap-6 bg-base-100 p-6">
      <span
        className="w-20 text-primary"
        role="img"
        aria-label="Monk Mode logo"
        dangerouslySetInnerHTML={{ __html: logo }}
      />
      <h1 className="font-serif text-4xl font-medium">Monk Mode</h1>
      <button className="btn btn-primary">
        <Hourglass size={18} aria-hidden="true" />
        Back to work.
      </button>
    </main>
  );
}
