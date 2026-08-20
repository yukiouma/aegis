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
}

/**
 * Single-pixel-high sentinel that calls `onIntersect` when it scrolls into
 * view. The parent owns `offset` and `hasMore`; this component is pure.
 */
export function InfiniteScrollSentinel({
  onIntersect,
  hasMore,
  loading,
  rootMargin = "0px 0px 200px 0px",
}: InfiniteScrollSentinelProps) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasMore) return;
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
      { rootMargin },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loading, onIntersect, rootMargin]);

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
