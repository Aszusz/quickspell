import { useCallback, useEffect, useRef, useState } from "react";

type Options = {
  estimatedItemHeight?: number;
  gap?: number;
  minItems?: number;
};

/**
 * Computes how many full items can fit vertically in a resizable container.
 * Returns refs for the container and a representative item so heights can be measured.
 */
export function usePaginationLayout({
  estimatedItemHeight = 44,
  gap = 0,
  minItems = 1,
}: Options = {}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [itemHeight, setItemHeight] = useState(estimatedItemHeight);
  const [pageSize, setPageSize] = useState(Math.max(1, minItems));

  const recompute = useCallback(() => {
    const container = containerRef.current;
    if (!container || itemHeight <= 0) return;

    const available = container.clientHeight;
    if (available <= 0) return;

    const next = Math.floor((available + gap) / (itemHeight + gap));
    const safeNext = Math.max(minItems, next || 1);

    setPageSize((prev) => (prev === safeNext ? prev : safeNext));
  }, [gap, itemHeight, minItems]);

  const measureItemRef = useCallback((node: HTMLElement | null) => {
    if (!node) return;
    const { height } = node.getBoundingClientRect();
    if (height > 0) {
      setItemHeight((prev) => (Math.abs(prev - height) > 0.5 ? height : prev));
    }
  }, []);

  useEffect(() => {
    recompute();
  }, [recompute, itemHeight]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let frame: number | null = null;
    const observer = new ResizeObserver(() => {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(recompute);
    });

    observer.observe(container);

    return () => {
      if (frame) cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [recompute]);

  return { containerRef, measureItemRef, pageSize } as const;
}
