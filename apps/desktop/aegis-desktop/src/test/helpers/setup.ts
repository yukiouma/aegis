import "@testing-library/jest-dom/vitest";

// jsdom does not implement window.scrollTo; TanStack Router's scroll
// restoration emits this on every render. Silence it.
window.scrollTo = () => undefined;

// jsdom does not implement IntersectionObserver. Provide a no-op shim so
// `new IntersectionObserver(cb, opts)` doesn't throw at construction time.
// Tests that need to simulate intersections (e.g. `InfiniteScrollSentinel`)
// replace this with a more capable mock in their own `beforeEach`.
class NoopIntersectionObserver implements IntersectionObserver {
  readonly root: Element | Document | null = null;
  readonly rootMargin = "0px";
  readonly thresholds: ReadonlyArray<number> = [];
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }
}
(globalThis as unknown as { IntersectionObserver: typeof IntersectionObserver }).IntersectionObserver =
  NoopIntersectionObserver;
