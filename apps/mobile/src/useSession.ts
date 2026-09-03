import { useCallback, useRef, useState } from "react";
import type { SessionResponse } from "@ldtg/shared";

export function useSessionState() {
  const [session, setSession] = useState<SessionResponse | null>(null);
  const sessionRef = useRef<SessionResponse | null>(null);

  const setCurrentSession = useCallback((next: SessionResponse) => {
    sessionRef.current = next;
    setSession(next);
  }, []);

  const clearSession = useCallback(() => {
    sessionRef.current = null;
    setSession(null);
  }, []);

  return { session, sessionRef, setCurrentSession, clearSession };
}
