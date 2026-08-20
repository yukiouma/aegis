import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { InfiniteScrollSentinel } from "./InfiniteScrollSentinel";

let observers: Array<{
  cb: IntersectionObserverCallback;
  observe: ReturnType<typeof vi.fn>;
  unobserve: ReturnType<typeof vi.fn>;
  disconnect: ReturnType<typeof vi.fn>;
}> = [];

beforeEach(() => {
  observers = [];
  const fakeObserver = class {
    cb: IntersectionObserverCallback;
    observe = vi.fn();
    unobserve = vi.fn();
    disconnect = vi.fn();
    constructor(cb: IntersectionObserverCallback) {
      this.cb = cb;
      observers.push(this);
    }
  };
  (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver =
    fakeObserver;
});

afterEach(() => {
  cleanup();
  observers = [];
});

function fireIntersect(idx: number, isIntersecting: boolean) {
  act(() => {
    observers[idx].cb(
      [{ isIntersecting } as IntersectionObserverEntry],
      observers[idx] as unknown as IntersectionObserver,
    );
  });
}

describe("InfiniteScrollSentinel", () => {
  it("calls onIntersect when intersection fires and hasMore=true, loading=false", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading={false} />);
    expect(observers).toHaveLength(1);
    fireIntersect(0, true);
    expect(onIntersect).toHaveBeenCalledTimes(1);
  });

  it("does not create an observer when hasMore=false", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore={false} loading={false} />);
    expect(observers).toHaveLength(0);
    // Nothing to fire — the component short-circuits to null.
    expect(onIntersect).not.toHaveBeenCalled();
  });

  it("does not call onIntersect while loading=true", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading />);
    fireIntersect(0, true);
    expect(onIntersect).not.toHaveBeenCalled();
  });

  it("renders a spinner while loading=true", () => {
    render(<InfiniteScrollSentinel onIntersect={() => {}} hasMore loading />);
    expect(screen.getByTestId("sentinel-spinner")).toBeInTheDocument();
  });

  it("disconnects the observer when hasMore flips to false", () => {
    const onIntersect = vi.fn();
    const { rerender } = render(
      <InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading={false} />,
    );
    const observer = observers[0];
    rerender(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore={false} loading={false} />);
    expect(observer.disconnect).toHaveBeenCalled();
  });
});
