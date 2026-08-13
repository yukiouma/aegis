import { useCallback, useRef, useState } from "react";

import type { LogEntry, PushLog } from "./types";

export interface BootstrapLogState {
  entries: LogEntry[];
  push: PushLog;
}

/**
 * Append-only log state for the bootstrap, login, and register pages.
 *
 * `push` is referentially stable, so callers can safely list it in a
 * `useEffect` dependency array without re-running the effect. Ids come
 * from a counter ref rather than the array index or a timestamp, so React
 * keys stay stable and unique.
 */
export function useBootstrapLog(): BootstrapLogState {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const nextId = useRef(0);

  const push = useCallback<PushLog>((level, key, params) => {
    const id = nextId.current;
    nextId.current += 1;
    setEntries((previous) => [...previous, { id, level, key, params }]);
  }, []);

  return { entries, push };
}
