# CLS (Cumulative Layout Shift) Improvements for PrediFi Frontend

## Overview

This document outlines the improvements made to reduce Cumulative Layout Shift (CLS) in the PrediFi frontend. CLS is a Core Web Vital metric that measures visual stability during page load and interaction. Lower CLS scores provide better user experience and improve SEO rankings.

## Problem Areas Identified and Fixed

### 1. **Image Dimension Issues**

#### Problem

Images without explicit dimensions can cause layout shift when they load.

#### Solutions Applied

**a) HeroSection Background Image** ([app/(marketing)/components/HeroSection.tsx](<app/(marketing)/components/HeroSection.tsx>))

- **Before**: Image with `fill` but in a non-positioned container
- **After**: Wrapped in an `absolute` positioned container with explicit size management
- **Impact**: Prevents layout shift as background image loads

**b) Features Section Images** ([app/(marketing)/components/Features.tsx](<app/(marketing)/components/Features.tsx>))

- **Before**: Used `width` and `height` props but responsive sizing caused shifts
- **After**: Wrapped in aspect ratio container (`aspect-square`) with `fill` mode

```tsx
<div className="relative w-full max-w-[180px] md:max-w-[400px] aspect-square">
  <Image
    src={feature.image}
    fill
    className="w-full h-auto object-contain"
    alt={feature.title}
  />
</div>
```

- **Benefits**:
  - Aspect ratio is locked from initial render
  - Image scales responsively without layout shift
  - Skeleton loading state maintains consistent height

### 2. **Dynamic Content Layout Shifts**

#### Problem

Dynamic content (modals, accordions, error messages) appearing/disappearing causes layout shift.

#### Solutions Applied

**a) Mobile Navigation Menu** ([app/(marketing)/components/NavBar.tsx](<app/(marketing)/components/NavBar.tsx>))

- **Before**: Conditional rendering with `{isOpen &&}` - menu appears/disappears causing shift
- **After**: Always rendered but with `maxHeight` and `opacity` transitions

```tsx
<div
  className="md:hidden overflow-hidden absolute top-full left-0 w-full..."
  style={{
    maxHeight: isOpen ? "500px" : "0px",
    opacity: isOpen ? 1 : 0,
  }}
>
  {/* Menu items always in DOM */}
</div>
```

- **Benefits**:
  - Smooth height transition without layout shift
  - Menu items prefetch even when hidden (performance)
  - Accessibility improved (items in DOM but hidden)

**b) Error Messages in Forms** ([components/Waitlist.tsx](components/Waitlist.tsx))

- **Before**: Error message conditionally rendered, causes space to appear/disappear
- **After**: Container with reserved minimum height

```tsx
<div
  style={{
    minHeight: "44px", // Reserve space for error message
    transition: "opacity 0.2s ease-out",
    opacity: status === "error" ? 1 : 0,
  }}
>
  {status === "error" && (
    <div className="rounded-lg bg-red-900/20...">{/* Error content */}</div>
  )}
</div>
```

- **Benefits**:
  - Space is reserved when form renders
  - Error appears with fade transition
  - No vertical shift when error appears

**c) FAQ Accordion** ([app/(marketing)/components/FAQ.tsx](<app/(marketing)/components/FAQ.tsx>))

- **Already Implemented**: Uses CSS Grid height transition technique
- **How it works**: Grid rows animate from `[0fr]` to `[1fr]` instead of height change

```css
grid-rows-[0fr] → grid-rows-[1fr]
```

- **Benefit**: Smooth, CLS-free accordion expansion

### 3. **Responsive Images with Aspect Ratio**

#### Problem

Responsive images without aspect ratio containers cause layout shift as they load on different screen sizes.

#### Solution

All feature images now use aspect ratio containers:

- Mobile: `max-w-[180px]` with `aspect-square`
- Desktop: `max-w-[400px]` with `aspect-square`
- Image scales within container without shifting layout

## Best Practices Applied

### 1. **Size Containers Before Content**

- Reserve space for dynamic content with `min-height`, `min-width`, or aspect ratio containers
- Don't conditionally render container - conditionally render content inside

### 2. **Use CSS Transitions, Not Conditional Rendering**

- For toggle-able content, use `maxHeight: 0` → `maxHeight: XXX` instead of conditional rendering
- For opacity changes, use `opacity: 0` → `opacity: 1`

### 3. **Aspect Ratio Containers**

- Wrap responsive images in containers with fixed aspect ratio
- Use Next.js `Image` component with `fill` and `object-contain`
- Lock aspect ratio from initial render

### 4. **Reserve Space for Async Content**

- Skeleton loaders should match final content dimensions
- Error messages should have reserved space
- Loading states should maintain layout

### 5. **Font Metrics Buffer**

- Next.js `next/font` with `display: "swap"` is already configured
- Prevents font-load induced CLS
- See [app/layout.tsx](app/layout.tsx) line 3-10

## Testing & Validation

### Web Vitals Measurement

To test CLS improvements:

1. **Local Testing**

   ```bash
   npm run dev
   # Open DevTools → Lighthouse
   # Run "Analyze page load"
   ```

2. **Production Testing**
   - Use [PageSpeed Insights](https://pagespeed.web.dev/)
   - Use [Web Vitals Chrome Extension](https://chrome.google.com/webstore/detail/web-vitals/ahfhijdlegdabiliapbnjehnhlnSKKL)

3. **Manual CLS Detection**
   - Open page in DevTools
   - Scroll and interact with dynamic content
   - Look for jumpy layouts
   - Check Console for CLS warnings

### Expected Improvements

- **HeroSection**: CLS reduction from background image stabilization
- **Features**: CLS reduction from image aspect ratio locking
- **NavBar**: CLS reduction from smooth menu transitions
- **Waitlist Form**: CLS reduction from error message space reservation
- **FAQ**: Already optimized with grid-based transitions

## Components Status

| Component   | CLS Issue              | Status          | Fix Type               |
| ----------- | ---------------------- | --------------- | ---------------------- |
| HeroSection | Background image shift | ✅ Fixed        | Container wrapping     |
| Features    | Image dimension shift  | ✅ Fixed        | Aspect ratio container |
| NavBar      | Menu appearance shift  | ✅ Fixed        | Max-height transition  |
| Waitlist    | Error message shift    | ✅ Fixed        | Min-height reservation |
| FAQ         | Accordion expansion    | ✅ Already Good | CSS Grid transition    |
| Dashboard   | Skeleton loading       | ✅ Already Good | Fixed dimensions       |
| MetricCard  | Loading state          | ✅ Already Good | Fixed dimensions       |

## Future Improvements

1. **Image Optimization**
   - Add blur-up placeholder for images
   - Use LQIP (Low Quality Image Placeholder) strategy
   - Consider next-gen formats (WebP, AVIF)

2. **Font Loading**
   - Monitor font load CLS impact
   - Consider `preload: true` for critical fonts

3. **Lazy Loading**
   - Ensure lazy-loaded content doesn't cause shifts
   - Use `loading="lazy"` with explicit dimensions

4. **Animation Optimization**
   - Use `transform` and `opacity` for smooth animations
   - Avoid animating properties that trigger layout

## Resources

- [Web Vitals Guide](https://web.dev/vitals/)
- [CLS Debugging Guide](https://web.dev/cls/)
- [Next.js Image Optimization](https://nextjs.org/docs/basic-features/image-optimization)
- [CSS Transitions for Performance](https://web.dev/animations-guide/)

## Contributing

When adding new dynamic content:

1. Reserve space for content before it loads/renders
2. Use CSS transitions instead of conditional rendering
3. Test with Lighthouse in DevTools
4. Verify CLS improvements before submitting PR

---

**Last Updated**: June 1, 2026
**Related PR**: Improve CLS for Dynamic Content
