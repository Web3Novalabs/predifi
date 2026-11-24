# Accessibility (WCAG 2.1 AA)

Covers [#1389](https://github.com/Web3Novalabs/predifi/issues/1389).

Target: **WCAG 2.1 Level AA**. This page records what the audit changed, the
primitives now available, and the checklist to apply when building new UI.

## Primitives

All exported from `@/components/ui`.

| Primitive | Use it for |
| --- | --- |
| `<SkipLink />` | Bypassing the nav. Rendered first in `app/layout.tsx`, so it is the first tab stop; visible only on focus. Targets `#main-content`. |
| `<VisuallyHidden>` | Context that sighted users get from layout or icons — icon-only button labels, table column meanings. Pass `focusable` to reveal on focus. |
| `<LiveRegionProvider>` / `useAnnounce()` | Announcing changes that do not move focus: prediction placed, claim confirmed, odds updated. Wraps the app in `layout.tsx`. |
| `<SkeletonScreen>` | Loading placeholders. Owns the single `role="status"` announcement so the individual bars stay `aria-hidden`. |

```tsx
const announce = useAnnounce();

await placePrediction(input);
announce("Prediction placed: 50 XLM on Yes");   // polite (default)
announce("Transaction failed", "assertive");     // interrupts
```

## What the audit changed

- **Bypass blocks (2.4.1)** — added `<SkipLink />` to the root layout and
  `id="main-content"` + `tabIndex={-1}` to the `<main>` landmark on the home,
  about, settings, and waitlist pages.
- **Status messages (4.1.3)** — added polite and assertive live regions so
  prediction and claim outcomes are announced without stealing focus.
- **Loading states (4.1.3, 1.3.1)** — skeletons announce once via
  `role="status"` + `aria-busy` instead of exposing dozens of empty nodes.
- **Motion (2.3.3)** — skeleton pulse and shimmer are behind Tailwind's
  `motion-safe:` prefix, so `prefers-reduced-motion: reduce` stops them.

## Checklist for new UI

**Keyboard (2.1.1, 2.4.3, 2.4.7)**
- [ ] Every interactive element is reachable and operable with Tab / Shift+Tab / Enter / Space.
- [ ] Tab order follows visual order; nothing is a keyboard trap.
- [ ] Focus is always visible — never remove the ring without replacing it (`focus-visible:ring-2`).
- [ ] Modals trap focus while open, close on Escape, and restore focus to the trigger.
- [ ] Custom controls built on `<div>` need `role`, `tabIndex={0}`, and Enter/Space handlers. Prefer a real `<button>`.

**Names and structure (1.3.1, 2.4.6, 4.1.2)**
- [ ] Icon-only buttons have `aria-label` or a `<VisuallyHidden>` child.
- [ ] Headings descend without skipping levels; one `<h1>` per page.
- [ ] Landmarks in place: `<header>`, `<nav>`, `<main id="main-content">`, `<footer>`.
- [ ] Form inputs have an associated `<label>`; errors use `aria-describedby` and `aria-invalid`.
- [ ] Data tables use `<th scope="col">` / `<th scope="row">`.

**Dynamic content (4.1.3)**
- [ ] Async results are announced via `useAnnounce()`.
- [ ] Loading regions set `aria-busy="true"`; placeholders are `aria-hidden`.
- [ ] Toasts render inside a live region.

**Colour and contrast (1.4.3, 1.4.11, 1.4.1)**
- [ ] Text contrast ≥ 4.5:1 (≥ 3:1 for text ≥ 24px or ≥ 19px bold).
- [ ] UI component and focus-indicator contrast ≥ 3:1.
- [ ] Colour is never the only signal — pair win/loss and up/down with an icon or text.

**Content and motion (1.4.4, 1.4.10, 2.3.3, 1.1.1)**
- [ ] Layout survives 200% zoom and a 320px viewport without horizontal scroll.
- [ ] Animation respects `prefers-reduced-motion` (`motion-safe:` / `motion-reduce:`).
- [ ] Images have `alt`; decorative images use `alt=""`.

## Verifying

```bash
cd frontend && pnpm lint
```

`eslint-config-next` includes `jsx-a11y` rules and catches missing alt text,
invalid ARIA attributes, and non-interactive elements with handlers.

Manual passes worth doing before shipping a flow:

1. Unplug the mouse. Complete the flow with the keyboard alone.
2. Run VoiceOver (`Cmd+F5` on macOS) through pool details and the prediction
   flow — every control should announce a name, a role, and its state.
3. Run Lighthouse or axe DevTools on `/`, `/dashboard`, and a pool detail page.
4. Zoom to 200% and check nothing is clipped or overlapping.

Automated tooling catches roughly a third of WCAG issues; the keyboard and
screen-reader passes are what find the rest.
