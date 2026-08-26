"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface ContainerSize {
  width: number;
  height: number;
}

/**
 * useContainerSize
 *
 * Tracks the pixel dimensions of a DOM element using ResizeObserver.
 * Returns a stable ref to attach to the element plus the current width/height.
 *
 * @example
 * const { ref, width, height } = useContainerSize();
 * return <div ref={ref} className="h-full w-full"><Chart width={width} height={height} /></div>
 */
export function useContainerSize<T extends HTMLElement = HTMLDivElement>(): {
  ref: React.RefCallback<T>;
  width: number;
  height: number;
} {
  const [size, setSize] = useState<ContainerSize>({ width: 0, height: 0 });
  const observerRef = useRef<ResizeObserver | null>(null);
  const elementRef = useRef<T | null>(null);

  const measure = useCallback((entry: ResizeObserverEntry) => {
    const { width, height } = entry.contentRect;
    setSize((prev) => {
      // Avoid re-renders when dimensions haven't actually changed
      if (prev.width === width && prev.height === height) return prev;
      return { width, height };
    });
  }, []);

  // Stable ref callback — wires/unwires the observer whenever the element changes
  const ref = useCallback(
    (node: T | null) => {
      // Disconnect from the previous element
      if (observerRef.current) {
        observerRef.current.disconnect();
        observerRef.current = null;
      }

      elementRef.current = node;

      if (!node) return;

      // Capture initial dimensions before the first ResizeObserver callback fires
      const rect = node.getBoundingClientRect();
      setSize({ width: rect.width, height: rect.height });

      // Watch for subsequent resizes
      observerRef.current = new ResizeObserver(([entry]) => {
        measure(entry);
      });
      observerRef.current.observe(node);
    },
    [measure],
  );

  // Disconnect on unmount
  useEffect(() => {
    return () => {
      observerRef.current?.disconnect();
    };
  }, []);

  return { ref, width: size.width, height: size.height };
}
