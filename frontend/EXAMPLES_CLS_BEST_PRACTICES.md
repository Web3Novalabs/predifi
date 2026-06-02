/\*\*

- CLS Best Practices Examples for PrediFi
-
- This file demonstrates how to use CLS utilities to build dynamic components
- that don't cause layout shift.
-
- Copy these patterns when adding new dynamic content!
  \*/

import {
ReservedSpace,
AspectRatioContainer,
SafeDropdown,
GridAccordionContent,
useLayoutSafeContainer,
} from "@/lib/cls-utils";
import { useState, useCallback } from "react";
import Image from "next/image";

// ============================================================================
// EXAMPLE 1: Form with Error Messages (CLS-Safe)
// ============================================================================

/\*\*

- Best practice: Reserve space for error message so form doesn't jump
  \*/
  export function FormWithErrorExample() {
  const [email, setEmail] = useState("");
  const [error, setError] = useState("");

const handleSubmit = (e: React.FormEvent) => {
e.preventDefault();
if (!email.includes("@")) {
setError("Invalid email address");
} else {
setError("");
}
};

return (
<form onSubmit={handleSubmit} className="space-y-6">
<div>
<input
type="email"
value={email}
onChange={(e) => setEmail(e.target.value)}
placeholder="Email"
className="w-full px-4 py-3 rounded-lg border border-gray-700"
/>
</div>

      {/* ❌ BAD: Conditionally render — causes layout shift */}
      {/* {error && (
        <div className="bg-red-900/20 border border-red-800 p-3 rounded-lg">
          {error}
        </div>
      )} */}

      {/* ✅ GOOD: Reserve space for error message */}
      <ReservedSpace isVisible={!!error} minHeight="60px">
        <div className="bg-red-900/20 border border-red-800 p-3 rounded-lg text-red-400">
          {error}
        </div>
      </ReservedSpace>

      <button type="submit" className="w-full px-4 py-2 bg-blue-600 rounded-lg">
        Submit
      </button>
    </form>

);
}

// ============================================================================
// EXAMPLE 2: Mobile Menu (CLS-Safe)
// ============================================================================

/\*\*

- Best practice: Always render menu in DOM, use CSS to show/hide
- This prevents layout shift when menu opens/closes
  \*/
  export function MobileMenuExample() {
  const [isOpen, setIsOpen] = useState(false);

return (
<nav className="bg-black">
<div className="flex items-center justify-between p-4">
<h1>Logo</h1>
<button onClick={() => setIsOpen(!isOpen)}>Menu</button>
</div>

      {/* ❌ BAD: Conditionally render menu */}
      {/* {isOpen && (
        <div className="space-y-4 p-4">
          <a href="/about">About</a>
          <a href="/features">Features</a>
        </div>
      )} */}

      {/* ✅ GOOD: Use SafeDropdown with CSS-based transitions */}
      <SafeDropdown isOpen={isOpen} maxHeight="400px" className="bg-gray-900">
        <div className="space-y-4 p-4">
          <a href="/about" className="block">
            About
          </a>
          <a href="/features" className="block">
            Features
          </a>
          <a href="/pricing" className="block">
            Pricing
          </a>
        </div>
      </SafeDropdown>
    </nav>

);
}

// ============================================================================
// EXAMPLE 3: Responsive Images with Aspect Ratio (CLS-Safe)
// ============================================================================

/\*\*

- Best practice: Lock aspect ratio before image loads
  _/
  export function ResponsiveImageExample() {
  return (
  <div className="grid grid-cols-2 gap-4">
  {/_ ❌ BAD: Image without aspect ratio container _/}
  {/_ <img src="/image.jpg" alt="Feature" className="w-full" /> \*/}

        {/* ✅ GOOD: Image with aspect ratio container */}
        <AspectRatioContainer aspectRatio="1 / 1" className="md:col-span-1">
          <Image
            src="/feature-image.jpg"
            alt="Feature"
            fill
            className="object-cover"
          />
        </AspectRatioContainer>

        {/* For 16:9 aspect ratio */}
        <AspectRatioContainer aspectRatio="16 / 9">
          <Image
            src="/hero-image.jpg"
            alt="Hero"
            fill
            className="object-cover"
          />
        </AspectRatioContainer>
      </div>

  );
  }

// ============================================================================
// EXAMPLE 4: Accordion (CLS-Safe)
// ============================================================================

/\*\*

- Best practice: Use CSS Grid transition for smooth height change
- This is already used in the FAQ component
  \*/
  export function AccordionExample() {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

const items = [
{ title: "Question 1", content: "Answer 1" },
{ title: "Question 2", content: "Answer 2" },
{ title: "Question 3", content: "Answer 3" },
];

return (
<div className="space-y-2">
{items.map((item, index) => (
<div key={index} className="border rounded-lg overflow-hidden">
<button
onClick={() => setOpenIndex(openIndex === index ? null : index)}
className="w-full p-4 text-left font-medium flex justify-between items-center" >
{item.title}
<span>{openIndex === index ? "−" : "+"}</span>
</button>

          {/* ❌ BAD: Conditionally render content */}
          {/* {openIndex === index && (
            <div className="p-4 border-t">
              {item.content}
            </div>
          )} */}

          {/* ✅ GOOD: Use GridAccordionContent for CLS-free expansion */}
          <GridAccordionContent isOpen={openIndex === index}>
            <div className="p-4 border-t">{item.content}</div>
          </GridAccordionContent>
        </div>
      ))}
    </div>

);
}

// ============================================================================
// EXAMPLE 5: Loading States (CLS-Safe)
// ============================================================================

/\*\*

- Best practice: Skeleton loader should match final content dimensions
  \*/
  export function LoadingStateExample() {
  const [isLoading, setIsLoading] = useState(true);

return (
<div>
{isLoading ? (
// ✅ GOOD: Skeleton matches final content dimensions
<AspectRatioContainer aspectRatio="16 / 9" className="rounded-lg overflow-hidden">
<div className="w-full h-full bg-gray-800 animate-pulse" />
</AspectRatioContainer>
) : (
// Final content with same aspect ratio
<AspectRatioContainer aspectRatio="16 / 9" className="rounded-lg overflow-hidden">
<Image
            src="/final-image.jpg"
            alt="Content"
            fill
            className="object-cover"
          />
</AspectRatioContainer>
)}
</div>
);
}

// ============================================================================
// EXAMPLE 6: Custom Hook Usage
// ============================================================================

/\*\*

- Using useLayoutSafeContainer hook for custom toggle behavior
  \*/
  export function CustomToggleExample() {
  const [isVisible, setIsVisible] = useState(false);
  const styles = useLayoutSafeContainer(isVisible, "300px", "250ms");

return (
<div>
<button onClick={() => setIsVisible(!isVisible)}>Toggle</button>

      {/* ✅ GOOD: Use hook for custom styling */}
      <div style={styles} className="bg-blue-100 p-4 rounded-lg">
        <p>This content expands and collapses smoothly!</p>
      </div>
    </div>

);
}

// ============================================================================
// EXAMPLE 7: Multi-State Dynamic Content (CLS-Safe)
// ============================================================================

/\*\*

- Complex example: Form with multiple states (loading, success, error)
- Each state must reserve space to prevent CLS
  \*/
  export function MultiStateFormExample() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">(
  "idle"
  );
  const [errorMessage, setErrorMessage] = useState("");

const handleSubmit = async (e: React.FormEvent) => {
e.preventDefault();
setStatus("loading");

    try {
      await new Promise((resolve) => setTimeout(resolve, 2000));
      setStatus("success");
    } catch {
      setErrorMessage("Something went wrong");
      setStatus("error");
    }

};

return (
<form onSubmit={handleSubmit} className="space-y-6 max-w-md">
<input
type="text"
placeholder="Enter text"
disabled={status === "loading"}
className="w-full px-4 py-3 rounded-lg border border-gray-700"
/>

      {/* Success Message — reserve space */}
      <ReservedSpace isVisible={status === "success"} minHeight="60px">
        <div className="bg-green-900/20 border border-green-800 p-3 rounded-lg text-green-400">
          ✓ Success! Your submission was received.
        </div>
      </ReservedSpace>

      {/* Error Message — reserve space */}
      <ReservedSpace isVisible={status === "error"} minHeight="60px">
        <div className="bg-red-900/20 border border-red-800 p-3 rounded-lg text-red-400">
          ✗ Error: {errorMessage}
        </div>
      </ReservedSpace>

      {/* Loading State — reserve space */}
      <ReservedSpace isVisible={status === "loading"} minHeight="20px">
        <div className="text-center text-sm text-gray-400">
          <span className="inline-block animate-spin mr-2">⟳</span>
          Submitting...
        </div>
      </ReservedSpace>

      <button
        type="submit"
        disabled={status === "loading"}
        className="w-full px-4 py-2 bg-blue-600 rounded-lg disabled:opacity-50"
      >
        {status === "loading" ? "Loading..." : "Submit"}
      </button>
    </form>

);
}

// ============================================================================
// KEY TAKEAWAYS
// ============================================================================

/\*\*

- 1.  Reserve Space, Don't Conditionally Render
- - Use ReservedSpace for fixed-size content
- - Use SafeDropdown for expandable menus
- - Use GridAccordionContent for accordion items
-
- 2.  Use Aspect Ratio Containers for Images
- - Lock aspect ratio before image loads
- - Prevents layout shift as image renders
- - Works with Next.js Image component
-
- 3.  Use CSS Transitions, Not Conditional Rendering
- - max-height: 0 → max-height: 500px
- - opacity: 0 → opacity: 1
- - Both always keep container in DOM
-
- 4.  Match Skeleton Dimensions to Final Content
- - Skeleton should be same size as loaded content
- - Prevents layout shift when content loads
-
- 5.  Test CLS with Lighthouse
- - DevTools → Lighthouse → Run audit
- - Check "Cumulative Layout Shift" metric
- - Target: CLS < 0.1 (good) or < 0.25 (needs improvement)
    \*/
