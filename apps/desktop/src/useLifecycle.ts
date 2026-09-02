import { useEffect, useRef } from "react";
import { errorMessage, invoke, listen } from "./tauriClient";

type LifecycleOptions = {
  dirty: boolean;
  running: boolean;
  snapshotAvailable: boolean;
  refreshService: () => Promise<void>;
  stop: (force?: boolean) => Promise<void>;
  quit: (force?: boolean, discardUnsaved?: boolean) => Promise<void>;
  onError: (message: string) => void;
};

export function useLifecycle({
  dirty,
  running,
  snapshotAvailable,
  refreshService,
  stop,
  quit,
  onError,
}: LifecycleOptions) {
  const allowUnload = useRef(false);

  useEffect(() => {
    if (!snapshotAvailable) return;
    void invoke("set_unsaved_changes", { dirty }).catch((error) => {
      onError(errorMessage(error));
    });
    // The native flag follows dirty-state transitions; snapshot hydration alone
    // intentionally does not emit a redundant command.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dirty]);

  useEffect(() => {
    if (!dirty) return;
    const preventUnload = (event: BeforeUnloadEvent) => {
      if (allowUnload.current) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", preventUnload);
    return () => window.removeEventListener("beforeunload", preventUnload);
  }, [dirty]);

  useEffect(() => {
    if (!running) return;
    let cancelled = false;
    let timer: number | undefined;
    const poll = async () => {
      await refreshService();
      if (!cancelled) timer = window.setTimeout(() => void poll(), 5000);
    };
    timer = window.setTimeout(() => void poll(), 5000);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [refreshService, running]);

  useEffect(() => {
    let refreshTimer: number | undefined;
    const scheduleServiceRefresh = () => {
      if (refreshTimer !== undefined) return;
      refreshTimer = window.setTimeout(() => {
        refreshTimer = undefined;
        void refreshService();
      }, 200);
    };
    const unlisteners = Promise.all([
      listen("stop-requested", () => void stop(false)),
      listen("quit-requested", () => void quit(false)),
      listen("service-status-changed", scheduleServiceRefresh),
      listen("sessions-changed", scheduleServiceRefresh),
      listen("transfer-updated", scheduleServiceRefresh),
      listen("network-changed", scheduleServiceRefresh),
    ]);
    return () => {
      if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
      void unlisteners.then((items) => items.forEach((unlisten) => unlisten()));
    };
    // Lifecycle commands intentionally retain the mount-time command closures,
    // matching the previous in-component listener behavior.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshService]);

  return allowUnload;
}
