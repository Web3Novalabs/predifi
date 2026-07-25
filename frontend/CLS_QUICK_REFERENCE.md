# CLS Quick Reference Guide

> **CLS = Cumulative Layout Shift** — A measure of visual stability during page load and interaction.
>
> **Target**: CLS < 0.1 (Good) | Acceptable: < 0.25 | Poor: > 0.25

## 🚫 Don't Do This

### Conditional Rendering of Content

```tsx
// BAD: Content appears/disappears causing shift
{
  isOpen && <div>Menu items</div>;
}
{
  error && <div>Error message</div>;
}
```

### Image Without Aspect Ratio

```tsx
// BAD: Size unknown until load
<img src="image.jpg" alt="..." className="w-full" />
<Image src="image.jpg" width={400} height={300} alt="..." />
```

### Skeleton Doesn't Match Content

```tsx
// BAD: Skeleton is smaller than final content
<Skeleton className="h-8 w-24" />;
{
  /* But loaded content is h-96 w-full! */
}
```

---

## ✅ Do This Instead

### Reserve Space with ReservedSpace

```tsx
import { ReservedSpace } from "@/lib/cls-utils";

// Space is reserved even when hidden
<ReservedSpace isVisible={!!error} minHeight="50px">
  <div className="error">{error}</div>
</ReservedSpace>;
```

### Use AspectRatioContainer for Images

```tsx
import { AspectRatioContainer } from "@/lib/cls-utils";
import Image from "next/image";

// Aspect ratio locked before load
<AspectRatioContainer aspectRatio="1 / 1">
  <Image src="image.jpg" fill alt="..." className="object-contain" />
</AspectRatioContainer>;
```

### Use SafeDropdown for Menus

```tsx
import { SafeDropdown } from "@/lib/cls-utils";

// Menu always in DOM, smooth transition
<SafeDropdown isOpen={isMenuOpen} maxHeight="400px">
  <a href="/page1">Page 1</a>
  <a href="/page2">Page 2</a>
</SafeDropdown>;
```

### Use GridAccordionContent for Accordions

```tsx
import { GridAccordionContent } from "@/lib/cls-utils";

// Accordion expands smoothly without CLS
<GridAccordionContent isOpen={isExpanded}>
  <div>Accordion content</div>
</GridAccordionContent>;
```

### Match Skeleton to Content

```tsx
// Skeleton should match final dimensions
const isLoading = true;

return isLoading ? (
  <div className="w-full h-[400px] bg-gray-800 animate-pulse" />
) : (
  <Image src="..." alt="..." width={800} height={400} />
);
```

---

## 📋 Checklist Before Pushing Code

- [ ] Does the page jump when loading?
- [ ] Does the page jump when scrolling?
- [ ] Does the page jump on user interaction (clicks, hovers)?
- [ ] Do images load with a size jump?
- [ ] Do modals/dropdowns appear smoothly?
- [ ] Do skeleton loaders match final content size?
- [ ] Does Lighthouse audit show CLS < 0.25?

---

## 🧪 Testing CLS

### 1. Manual Test in DevTools

```
1. Open page in Chrome
2. F12 → Lighthouse tab
3. Click "Analyze page load"
4. Check "Cumulative Layout Shift" score
```

### 2. Use Web Vitals Extension

- [Install](https://chrome.google.com/webstore/detail/web-vitals/) Web Vitals Chrome extension
- Shows live CLS, LCP, FID on every page

### 3. Test Specific Interactions

```
1. Load page
2. Scroll slowly - watch for jumps
3. Click buttons/links
4. Resize browser window
5. Interact with forms
```

---

## 🔄 Common Patterns

### Pattern 1: Form with Error Message

```tsx
const [error, setError] = useState("");

return (
  <form>
    <input type="email" />

    {/* Reserve space for error */}
    <ReservedSpace isVisible={!!error} minHeight="50px">
      <div className="error">{error}</div>
    </ReservedSpace>

    <button>Submit</button>
  </form>
);
```

### Pattern 2: Mobile Navigation Menu

```tsx
const [isOpen, setIsOpen] = useState(false);

return (
  <nav>
    <button onClick={() => setIsOpen(!isOpen)}>Menu</button>

    {/* Smooth height transition */}
    <SafeDropdown isOpen={isOpen} maxHeight="500px">
      <a href="/about">About</a>
      <a href="/features">Features</a>
    </SafeDropdown>
  </nav>
);
```

### Pattern 3: Image Grid

```tsx
const images = [
  { src: "img1.jpg", alt: "Image 1" },
  { src: "img2.jpg", alt: "Image 2" },
];

return (
  <div className="grid grid-cols-3 gap-4">
    {images.map((img) => (
      <AspectRatioContainer key={img.src} aspectRatio="1 / 1">
        <Image src={img.src} fill alt={img.alt} className="object-cover" />
      </AspectRatioContainer>
    ))}
  </div>
);
```

### Pattern 4: Loading State

```tsx
const { data, isLoading } = useData();

if (isLoading) {
  return <Skeleton width="100%" height="400px" className="rounded-lg" />;
}

return (
  <AspectRatioContainer aspectRatio="16 / 9">
    <Image src={data.image} fill alt="data" />
  </AspectRatioContainer>
);
```

---

## 📚 API Reference

### useLayoutSafeContainer()

```tsx
const styles = useLayoutSafeContainer(
  isVisible: boolean,
  maxHeight?: string = "500px",
  duration?: string = "200ms"
);

<div style={styles}>{content}</div>
```

### ReservedSpace

```tsx
<ReservedSpace isVisible={boolean} minHeight="44px" children={ReactNode} />
```

### AspectRatioContainer

```tsx
<AspectRatioContainer
  aspectRatio="1 / 1"
  className="rounded-lg"
  children={ReactNode}
/>
```

### SafeDropdown

```tsx
<SafeDropdown
  isOpen={boolean}
  maxHeight="500px"
  className="bg-white"
  children={ReactNode}
/>
```

### GridAccordionContent

```tsx
<GridAccordionContent
  isOpen={boolean}
  transitionDuration="300ms"
  children={ReactNode}
/>
```

### SafeModal

```tsx
<SafeModal
  isOpen={boolean}
  backgroundColor="rgba(0,0,0,0.5)"
  children={ReactNode}
/>
```

---

## 🆘 Troubleshooting

### Q: Page still jumping after using ReservedSpace?

**A**: Check `minHeight` is large enough for content. Add 10-20px buffer.

### Q: Aspect ratio container showing black box?

**A**: Make sure `position: relative` is on container (it is by default), and Image has `fill` prop.

### Q: SafeDropdown still causes shift?

**A**: Increase `maxHeight` prop if menu content is taller.

### Q: How do I know my CLS is good?

**A**: Run Lighthouse audit → CLS should be < 0.1. Anything > 0.25 is poor.

### Q: Can I use conditional rendering sometimes?

**A**: Yes, but only for completely out-of-flow content (modals, overlays). For in-flow content, always reserve space.

---

## 📖 Learn More

- **Full Guide**: [CLS_IMPROVEMENTS.md](../CLS_IMPROVEMENTS.md)
- **Examples**: [EXAMPLES_CLS_BEST_PRACTICES.md](../EXAMPLES_CLS_BEST_PRACTICES.md)
- **Tests**: [**tests**/cls.test.ts](__tests__/cls.test.ts)
- **Google Web Vitals**: https://web.dev/vitals/
- **CLS Debugging**: https://web.dev/cls/

---

**Last Updated**: June 1, 2026
**Version**: 1.0
