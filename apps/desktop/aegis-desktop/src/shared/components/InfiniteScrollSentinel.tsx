import { Box, CircularProgress } from "@aegis/ui/mui";
import { useEffect, useRef } from "react";

export interface InfiniteScrollSentinelProps {
  /** Called when the sentinel scrolls into view and `hasMore && !loading`. */
  onIntersect: () => void;
  /** Stop firing `onIntersect` when false. */
  hasMore: boolean;
  /** Suppress `onIntersect` while a page fetch is in flight. */
  loading: boolean;
  /** Pixel margin before the viewport edge at which the observer fires. */
  rootMargin?: string;
  /** IntersectionObserver root. When set, the observer fires based on this
   *  element's visibility instead of the viewport. Use when the sentinel
   *  lives inside a scroll container that scrolls independently of the
   *  page (e.g. a scrollable MUI TableContainer). */
  root?: Element | null;
}

/**
 * Single-pixel-high sentinel that calls `onIntersect` when it scrolls into
 * view. The parent owns `offset` and `hasMore`; this component is pure.
 *
 * When `root` is omitted, the observer uses the viewport. When `root` is an
 * `Element`, the observer fires based on visibility inside that element's
 * scroll box — pair with the sentinel being rendered *inside* the scroll
 * container. When `root` is `null`, no observer is created (handles the
 * first render before a parent's callback ref resolves).
 */
export function InfiniteScrollSentinel({
  onIntersect,
  hasMore,
  loading,
  rootMargin = "0px 0px 200px 0px",
  root,
}: InfiniteScrollSentinelProps) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasMore) return;
    // Distinguish `root === undefined` (default: viewport) from `root === null`
    // (explicitly null: parent callback ref hasn't resolved yet — wait).
    if (root === null) return;
    const el = ref.current;
    if (el == null) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && !loading) {
            onIntersect();
            break;
          }
        }
      },
      // `undefined` here is fine: IntersectionObserver defaults to viewport.
      { root, rootMargin },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loading, onIntersect, rootMargin, root]);

  if (!hasMore) return null;

  return (
    <Box
      ref={ref}
      sx={{
        display: "flex",
        justifyContent: "center",
        py: 1,
        minHeight: 8,
      }}
      data-testid="infinite-scroll-sentinel"
    >
      {loading ? (
        <Box data-testid="sentinel-spinner">
          <CircularProgress size={20} />
        </Box>
      ) : null}
    </Box>
  );
}