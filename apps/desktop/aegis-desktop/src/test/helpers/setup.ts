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

// jsdom does not implement ResizeObserver. @dnd-kit/react requires it at
// module-load time. Shim with a no-op so DragDropProvider can mount in tests.
class NoopResizeObserver implements ResizeObserver {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
(globalThis as unknown as { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
  NoopResizeObserver;

// jsdom does not implement PointerEvent. @dnd-kit/dom checks
// `event instanceof PointerEvent` when a click happens on a draggable
// element; without a global `PointerEvent` constructor that `instanceof`
// throws. A no-op class is enough — dnd-kit only checks the type.
class NoopPointerEvent extends MouseEvent {
  readonly pointerId: number;
  readonly pointerType: string;
  constructor(type: string, params?: PointerEventInit) {
    super(type, params);
    this.pointerId = params?.pointerId ?? 0;
    this.pointerType = params?.pointerType ?? "";
  }
}
(globalThis as unknown as { PointerEvent: typeof PointerEvent }).PointerEvent =
  NoopPointerEvent as unknown as typeof PointerEvent;
