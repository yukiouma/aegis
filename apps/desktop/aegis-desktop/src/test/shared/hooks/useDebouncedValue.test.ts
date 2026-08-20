import "@testing-library/jest-dom/vitest";
import { act, renderHook } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

/** Counts the number of times `settled` changes (i.e. the hook fires). */
function makeFireCounter() {
  const ref = { current: 0 };
  return ref;
}

function useFireCounter(
  settled: unknown,
  initialValue: unknown,
  counter: { current: number },
) {
  useEffect(() => {
    if (settled !== initialValue) {
      counter.current++;
    }
  }, [settled, initialValue, counter]);
}

describe("useDebouncedValue", () => {
  it("returns the initial value on first render", () => {
    const { result } = renderHook(() =>
      useDebouncedValue("a", { delayMs: 300, maxWaitMs: 1000 }),
    );
    expect(result.current).toBe("a");
  });

  it("emits the trailing value after delayMs when input stops changing", () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "a" } },
    );

    rerender({ value: "ab" });
    rerender({ value: "abc" });
    expect(result.current).toBe("a"); // not yet

    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(result.current).toBe("abc");
  });

  it("throttles continuous changes to at most one fire per maxWaitMs", () => {
    const fires = makeFireCounter();
    const { rerender } = renderHook(
      ({ value }: { value: number }) => {
        const settled = useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 });
        useFireCounter(settled, 0, fires);
        return settled;
      },
      { initialProps: { value: 0 } },
    );

    // 20 changes × 200ms = 4000ms wall-clock. With maxWaitMs = 1000, we expect
    // at least 3 and at most 5 fires (one per second, plus trailing).
    for (let i = 1; i <= 20; i++) {
      rerender({ value: i });
      act(() => {
        vi.advanceTimersByTime(200);
      });
    }
    expect(fires.current).toBeGreaterThanOrEqual(3);
    expect(fires.current).toBeLessThanOrEqual(5);
  });

  it("does not emit when the value is unchanged across renders", () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "x" } },
    );
    rerender({ value: "x" });
    rerender({ value: "x" });
    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(result.current).toBe("x");
  });

  it("cancels pending timers on unmount", () => {
    const { rerender, unmount } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "a" } },
    );
    rerender({ value: "b" });
    unmount();
    expect(() => vi.advanceTimersByTime(1000)).not.toThrow();
  });
});
