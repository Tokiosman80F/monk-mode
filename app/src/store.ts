import { useEffect } from "react";
import { create } from "zustand";
import { api, errorText } from "./api";
import type { Snapshot } from "./types/Snapshot";

interface ServiceStore {
  snapshot: Snapshot | null;
  /** Null until the first answer, so the banner doesn't flash at startup. */
  connected: boolean | null;
  setSnapshot: (snapshot: Snapshot) => void;
  setConnected: (connected: boolean) => void;
}

export const useStore = create<ServiceStore>((set) => ({
  snapshot: null,
  connected: null,
  // Hearing from the service at all means it's up.
  setSnapshot: (snapshot) => set({ snapshot, connected: true }),
  setConnected: (connected) => set({ connected }),
}));

/** Keeps the store in step with the service. Call once, at the top of the app. */
export function useServiceState() {
  useEffect(() => {
    const { setSnapshot, setConnected } = useStore.getState();
    const unlisten = [
      api.onStateChanged(setSnapshot),
      api.onServiceStatus(setConnected),
    ];
    api
      .getState()
      .then(setSnapshot)
      .catch(() => setConnected(false));
    return () => {
      unlisten.forEach((promise) => void promise.then((stop) => stop()));
    };
  }, []);
}

/** Runs a call that returns fresh state, and returns the error text if it fails. */
export async function applyCall(
  call: Promise<Snapshot>,
): Promise<string | null> {
  try {
    useStore.getState().setSnapshot(await call);
    return null;
  } catch (error) {
    return errorText(error);
  }
}
