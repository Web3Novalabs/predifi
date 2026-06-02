/**
 * CLS (Cumulative Layout Shift) utilities for PrediFi frontend
 * 
 * Provides hooks and utilities to prevent layout shift from dynamic content.
 * 
 * Usage:
 *   - Use useLayoutSafeContainer() for dynamic visibility toggles
 *   - Use AspectRatioImage component for responsive images
 *   - Use ReservedSpace component for space reservation
 */

import { ReactNode, CSSProperties } from 'react';

/**
 * Hook for CSS-based visibility with no layout shift
 * 
 * Returns inline styles for safe show/hide transitions
 * 
 * @param isVisible - whether content should be shown
 * @param maxHeight - maximum height when visible (default: "500px")
 * @param transitionDuration - CSS transition duration (default: "200ms")
 * 
 * @example
 * const navStyles = useLayoutSafeContainer(isOpen, "600px");
 * <div style={navStyles}>{/* content */}</div>
 */
export function useLayoutSafeContainer(
  isVisible: boolean,
  maxHeight: string = "500px",
  transitionDuration: string = "200ms"
): CSSProperties {
  return {
    overflow: "hidden",
    maxHeight: isVisible ? maxHeight : "0px",
    opacity: isVisible ? 1 : 0,
    transition: `max-height ${transitionDuration} ease-out, opacity ${transitionDuration} ease-out`,
  };
}

/**
 * Component that reserves vertical space for dynamic content
 * 
 * Prevents layout shift when content appears/disappears with fade transition
 * 
 * @param isVisible - whether content is shown
 * @param minHeight - space to reserve (default: "44px")
 * @param children - content to show/hide
 * 
 * @example
 * <ReservedSpace minHeight="44px" isVisible={hasError}>
 *   <div className="error">Error message</div>
 * </ReservedSpace>
 */
export function ReservedSpace({
  isVisible,
  minHeight = "44px",
  children,
}: {
  isVisible: boolean;
  minHeight?: string;
  children: ReactNode;
}): JSX.Element {
  return (
    <div
      style={{
        minHeight,
        transition: "opacity 0.2s ease-out",
        opacity: isVisible ? 1 : 0,
      }}
    >
      {children}
    </div>
  );
}

/**
 * Aspect ratio container for responsive images
 * 
 * Locks image aspect ratio to prevent layout shift as image loads
 * 
 * @param aspectRatio - CSS aspect ratio (default: "1 / 1")
 * @param children - Image or media element
 * @param className - additional classes
 * 
 * @example
 * <AspectRatioContainer aspectRatio="16 / 9">
 *   <Image src="..." fill alt="..." />
 * </AspectRatioContainer>
 */
export function AspectRatioContainer({
  aspectRatio = "1 / 1",
  children,
  className = "",
}: {
  aspectRatio?: string;
  children: ReactNode;
  className?: string;
}): JSX.Element {
  return (
    <div
      className={`relative w-full ${className}`}
      style={{
        aspectRatio,
      }}
    >
      {children}
    </div>
  );
}

/**
 * Skeleton loader with explicit dimensions to match content
 * 
 * @param width - CSS width
 * @param height - CSS height
 * @param className - additional Tailwind classes
 * 
 * @example
 * <CLSSafeSkeleton width="100%" height="400px" className="rounded-lg" />
 */
export function CLSSafeSkeleton({
  width = "100%",
  height = "100px",
  className = "",
}: {
  width?: string;
  height?: string;
  className?: string;
}): JSX.Element {
  return (
    <div
      className={`animate-pulse rounded-md bg-zinc-800/60 ${className}`}
      style={{
        width,
        height,
      }}
    />
  );
}

/**
 * Safe dropdown/modal container that doesn't cause layout shift
 * 
 * Always renders the container, just hides/shows content with CSS
 * 
 * @param isOpen - whether dropdown/modal is visible
 * @param children - dropdown/modal content
 * @param maxHeight - max height when open (default: "500px")
 * @param className - additional classes
 * 
 * @example
 * <SafeDropdown isOpen={isMenuOpen}>
 *   <a href="/about">About</a>
 *   <a href="/features">Features</a>
 * </SafeDropdown>
 */
export function SafeDropdown({
  isOpen,
  children,
  maxHeight = "500px",
  className = "",
}: {
  isOpen: boolean;
  children: ReactNode;
  maxHeight?: string;
  className?: string;
}): JSX.Element {
  const styles = useLayoutSafeContainer(isOpen, maxHeight);
  
  return (
    <div style={styles} className={`overflow-hidden ${className}`}>
      {children}
    </div>
  );
}

/**
 * Grid-based accordion item for CLS-free expansion
 * 
 * Uses CSS Grid trick: `grid-rows-[0fr]` → `grid-rows-[1fr]` for smooth height change
 * 
 * @param isOpen - whether accordion is expanded
 * @param children - accordion content
 * @param transitionDuration - CSS transition duration (default: "300ms")
 * 
 * @example
 * <GridAccordionContent isOpen={isExpanded}>
 *   <p>Details go here</p>
 * </GridAccordionContent>
 */
export function GridAccordionContent({
  isOpen,
  children,
  transitionDuration = "300ms",
}: {
  isOpen: boolean;
  children: ReactNode;
  transitionDuration?: string;
}): JSX.Element {
  return (
    <div
      className={`grid transition-[grid-template-rows] ${
        isOpen ? "grid-rows-[1fr]" : "grid-rows-[0fr]"
      }`}
      style={{
        transitionDuration,
      }}
    >
      <div className="overflow-hidden">{children}</div>
    </div>
  );
}

/**
 * Safe modal overlay that reserves space without layout shift
 * 
 * @param isOpen - whether modal is visible
 * @param children - modal content
 * @param backgroundColor - overlay background (default: "rgba(0,0,0,0.5)")
 * 
 * @example
 * <SafeModal isOpen={showModal}>
 *   <div className="modal-content">...</div>
 * </SafeModal>
 */
export function SafeModal({
  isOpen,
  children,
  backgroundColor = "rgba(0,0,0,0.5)",
}: {
  isOpen: boolean;
  children: ReactNode;
  backgroundColor?: string;
}): JSX.Element {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor,
        opacity: isOpen ? 1 : 0,
        pointerEvents: isOpen ? "auto" : "none",
        transition: "opacity 0.2s ease-out",
        zIndex: 50,
      }}
    >
      {children}
    </div>
  );
}
