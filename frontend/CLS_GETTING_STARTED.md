# CLS for New Contributors – Getting Started Guide

Welcome to PrediFi! This guide helps you understand and apply CLS (Cumulative Layout Shift) best practices to avoid contributing code that causes layout instability.

## 🎯 What is CLS and Why Should I Care?

**CLS** measures how much the page jumps around while loading and during interaction.

- **Good CLS**: < 0.1 — Feels smooth and professional
- **Acceptable**: < 0.25 — Users might notice small jumps
- **Poor**: > 0.25 — Noticeable jank, bad user experience

CLS impacts:

- User experience (direct impact on satisfaction)
- SEO rankings (Google uses it for ranking)
- Core Web Vitals score (business metrics)

---

## 🚀 Quick Start: 5-Minute Overview

### The Rule: Reserve Space Before Content

```tsx
// ❌ DON'T: Content appears/disappears causing shift
{
  isOpen && <Menu>items</Menu>;
}

// ✅ DO: Space is reserved, content fades in
<ReservedSpace isVisible={isOpen} minHeight="200px">
  <Menu>items</Menu>
</ReservedSpace>;
```

### The Strategy: CSS Transitions, Not Conditional Rendering

```tsx
// ❌ DON'T: DOM changes cause layout recalculation
const style = isOpen ? { display: "block" } : { display: "none" };

// ✅ DO: CSS changes don't trigger layout (just repaints)
const style = {
  maxHeight: isOpen ? "500px" : "0px",
  opacity: isOpen ? 1 : 0,
};
```

### The Tools: Use CLS Utilities

```tsx
import { ReservedSpace, AspectRatioContainer, SafeDropdown } from "@/lib/cls-utils";

// Space reservation for dynamic content
<ReservedSpace isVisible={hasError} minHeight="50px">
  <div className="error">{error}</div>
</ReservedSpace>

// Aspect ratio container for images
<AspectRatioContainer aspectRatio="16 / 9">
  <Image src="..." fill alt="..." />
</AspectRatioContainer>

// Safe menu dropdowns
<SafeDropdown isOpen={isMenuOpen} maxHeight="400px">
  <a href="/page">Link</a>
</SafeDropdown>
```

---

## 📖 Learning Path

### Level 1: Understand the Problem (5 min read)

**Read**: `CLS_QUICK_REFERENCE.md`

**You'll learn**:

- What causes CLS
- What not to do
- How to test CLS
- Common patterns

### Level 2: See Working Examples (10 min read)

**Read**: `EXAMPLES_CLS_BEST_PRACTICES.md`

**You'll learn**:

- 7 real working examples
- Copy-paste ready code
- How to apply patterns to your components

### Level 3: Deep Dive into Implementation (15 min read)

**Read**: `CLS_IMPROVEMENTS.md`

**You'll learn**:

- Technical details of fixes
- Reasoning behind each change
- Testing and validation methods
- Future improvements

### Level 4: Use the API Reference (5 min lookup)

**See**: `frontend/lib/cls-utils.ts`

**You'll find**:

- All available components
- Function signatures
- TypeScript types
- Usage examples

---

## 🛠️ Common Tasks

### Task 1: I'm Adding a Form with Validation

```tsx
import { ReservedSpace } from "@/lib/cls-utils";
import { useState } from "react";

export function MyForm() {
  const [error, setError] = useState("");

  return (
    <form>
      <input type="email" />

      {/* Reserve space for error */}
      <ReservedSpace isVisible={!!error} minHeight="50px">
        <div className="error-message">{error}</div>
      </ReservedSpace>

      <button>Submit</button>
    </form>
  );
}
```

**Why**: Form height changes when error appears → causes CLS

**Solution**: Minheight reserves space, error fades in without shift

---

### Task 2: I'm Adding Responsive Images

```tsx
import { AspectRatioContainer } from "@/lib/cls-utils";
import Image from "next/image";

export function MyGallery() {
  return (
    <div className="grid grid-cols-3 gap-4">
      {images.map((img) => (
        <AspectRatioContainer key={img.id} aspectRatio="1 / 1">
          <Image src={img.src} fill alt={img.alt} className="object-cover" />
        </AspectRatioContainer>
      ))}
    </div>
  );
}
```

**Why**: Images load at unknown size → dimensions unknown until loaded → CLS when they appear

**Solution**: Aspect ratio container locks dimensions before load

---

### Task 3: I'm Adding a Mobile Menu

```tsx
import { SafeDropdown } from "@/lib/cls-utils";
import { useState } from "react";

export function MobileNav() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <nav>
      <button onClick={() => setIsOpen(!isOpen)}>Menu</button>

      <SafeDropdown isOpen={isOpen} maxHeight="400px">
        <a href="/about">About</a>
        <a href="/features">Features</a>
        <a href="/pricing">Pricing</a>
      </SafeDropdown>
    </nav>
  );
}
```

**Why**: Menu appears/disappears → content below shifts → CLS

**Solution**: CSS-based height transition, menu always in DOM

---

### Task 4: I'm Adding an Accordion

```tsx
import { GridAccordionContent } from "@/lib/cls-utils";
import { useState } from "react";

export function FAQ() {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <div>
      {faqs.map((faq, index) => (
        <div key={index}>
          <button
            onClick={() => setOpenIndex(openIndex === index ? null : index)}
          >
            {faq.question}
          </button>

          <GridAccordionContent isOpen={openIndex === index}>
            <div>{faq.answer}</div>
          </GridAccordionContent>
        </div>
      ))}
    </div>
  );
}
```

**Why**: Accordion height changes during expand → CLS

**Solution**: CSS Grid transition handles height smoothly

---

### Task 5: I'm Adding a Modal

```tsx
import { SafeModal } from "@/lib/cls-utils";

export function MyModal() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <>
      <button onClick={() => setIsOpen(true)}>Open Modal</button>

      <SafeModal isOpen={isOpen} backgroundColor="rgba(0,0,0,0.5)">
        <div className="modal-content">
          <p>Modal content</p>
          <button onClick={() => setIsOpen(false)}>Close</button>
        </div>
      </SafeModal>
    </>
  );
}
```

**Why**: Modal is out-of-flow so conditional rendering is OK, but content can fade for smoothness

**Solution**: SafeModal provides smooth fade in/out

---

## ✅ Checklist: Before I Submit a PR

- [ ] **No conditional rendering of in-flow content**
  - Use ReservedSpace instead
  - Use SafeDropdown instead
  - Use GridAccordionContent instead

- [ ] **All images have aspect ratio containers**
  - Use AspectRatioContainer
  - Or use CSS aspect-ratio property

- [ ] **Skeleton loaders match content size**
  - Skeleton height = final content height
  - Skeleton width = final content width

- [ ] **Form errors don't cause shift**
  - Error message container has fixed height
  - Use ReservedSpace

- [ ] **Tested with Lighthouse**
  - Run DevTools Lighthouse audit
  - CLS < 0.25 (target < 0.1)

- [ ] **No console warnings or errors**
  - Check DevTools console
  - Run TypeScript type check: `npm run type-check`

---

## 🧪 Quick CLS Test

### Before Submitting PR:

1. **Start dev server**

   ```bash
   pnpm dev
   ```

2. **Open DevTools** (F12)

3. **Run Lighthouse audit**
   - Lighthouse tab → Analyze page load
   - Check "Cumulative Layout Shift" metric

4. **Manual interaction test**
   - Scroll the page slowly
   - Click buttons/links
   - Watch for jumps/shifts

5. **Check result**
   - CLS < 0.1 ✅ Perfect
   - CLS < 0.25 ✅ Acceptable
   - CLS > 0.25 ❌ Needs fix

---

## 🆘 Help! I'm Stuck

### I can't find the right utility

→ Check `frontend/lib/cls-utils.ts` for all available components

### I need a code example

→ See `EXAMPLES_CLS_BEST_PRACTICES.md` for 7 working examples

### I don't understand why something causes CLS

→ Read `CLS_IMPROVEMENTS.md` section "Problem Areas Identified"

### I want to learn CLS deeply

→ Visit https://web.dev/cls/ (Google's official guide)

### I need help from the team

→ Ask in the PR review or Telegram community

---

## 🎓 Further Learning

### Official Resources

- [Google Web Vitals Guide](https://web.dev/vitals/)
- [CLS Explanation](https://web.dev/cls/)
- [Web Vitals YouTube](https://www.youtube.com/watch?v=AQqFZ5t8uNc)

### PrediFi Resources

- `CLS_QUICK_REFERENCE.md` — Quick lookup
- `EXAMPLES_CLS_BEST_PRACTICES.md` — Code examples
- `CLS_IMPROVEMENTS.md` — Deep technical dive
- `frontend/lib/cls-utils.ts` — API reference

### Measure CLS

- [PageSpeed Insights](https://pagespeed.web.dev/) — Production measurement
- [Web Vitals Extension](https://chrome.google.com/webstore/detail/web-vitals/) — Live measurement
- Lighthouse DevTools — Local testing

---

## 💡 Pro Tips

### Tip 1: Test While You Code

Don't wait until the end. Run Lighthouse while developing.

### Tip 2: Think About Space First

Before adding any dynamic content, ask: "Where should this space come from?"

### Tip 3: Use CSS, Not JavaScript

CSS transitions don't cause layout recalculation. DOM changes do.

### Tip 4: Keep Skeletons Honest

Skeleton should be exact same size as final content.

### Tip 5: Test on Real Devices

Mobile devices can have worse CLS than desktop.

---

## 🚀 You're Ready!

You now understand CLS and have the tools to avoid it. When you code, remember:

> **Reserve space before content loads.** Use CSS transitions, not conditional rendering. Test with Lighthouse.

Questions? Ask the team! Happy coding! 🎉

---

**Next Steps**:

1. Read the Quick Reference guide (`CLS_QUICK_REFERENCE.md`)
2. Look at the examples (`EXAMPLES_CLS_BEST_PRACTICES.md`)
3. Start your first feature with CLS in mind
4. Run Lighthouse before submitting PR
5. Share your learnings with the team!

---

_Welcome to PrediFi! Let's build a smooth, stable UI together._ 🚀
