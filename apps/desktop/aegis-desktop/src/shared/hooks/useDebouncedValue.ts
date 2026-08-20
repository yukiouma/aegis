import { useEffect, useRef, useState } from "react";

export interface UseDebouncedValueOptions {
  /** Trailing-edge debounce window after the last change. */
  delayMs: number;
  /** Maximum time to wait between fires while the value is still changing. */
  maxWaitMs: number;
}

/**
 * Returns a "settled" value that lags behind `value` until either the
 * trailing-debounce window (`delayMs`) or the max-wait window (`maxWaitMs`)
 * has elapsed — whichever comes first. The max-wait timer is anchored to the
 * start of each window (the first input change after a fire), so sustained
 * rapid input yields one fire per `maxWaitMs` even if `delayMs` never lands.
 *
 * See `docs/superpowers/specs/2026-08-20-terminology-page-pagination-debounce-design.md`
 * section 7 for the exact semantics.
 */
export function useDebouncedValue<T>(
  value: T,
  options: UseDebouncedValueOptions,
): T {
  const { delayMs, maxWaitMs } = options;
  const [settled, setSettled] = useState<T>(value);
  const latestRef = useRef<T>(value);
  const delayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const maxTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    latestRef.current = value;

    const fire = () => {
      setSettled(latestRef.current);
      if (delayTimerRef.current != null) {
        clearTimeout(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      if (maxTimerRef.current != null) {
        clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
    };

    // Always (re)start the trailing timer — it tracks "300ms after the last
    // change". Cancelling on cleanup is correct because each new change
    // resets the trailing window.
    if (delayTimerRef.current != null) clearTimeout(delayTimerRef.current);
    delayTimerRef.current = setTimeout(fire, delayMs);

    // Only start the maxWait timer once per window. Subsequent input changes
    // within the same window leave it alone — that's what gives us the
    // "at most one fire per maxWaitMs" cap during sustained input.
    if (maxTimerRef.current == null) {
      maxTimerRef.current = setTimeout(fire, maxWaitMs);
    }

    return () => {
      if (delayTimerRef.current != null) {
        clearTimeout(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      // maxTimerRef intentionally NOT cleared — it persists across input
      // changes within the window.
    };
  }, [value, delayMs, maxWaitMs]);

  // Unmount cleanup — clear both timers.
  useEffect(() => {
    return () => {
      if (delayTimerRef.current != null) {
        clearTimeout(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      if (maxTimerRef.current != null) {
        clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
    };
  }, []);

  return settled;
}
