# UI Components Library

A comprehensive library of reusable UI components for the PrediFi Dapp, built with shadcn/ui and customized to match the Figma design specifications.

## 📦 Components Overview

All components are built with:
- **shadcn/ui** as the base foundation
- **Radix UI primitives** for accessibility
- **Tailwind CSS** for styling
- **TypeScript** for type safety
- Full **accessibility** support (ARIA labels, keyboard navigation)
- Each component under **150 lines** of code

## 🎨 Components

### Button Component
**File:** `components/ui/button.tsx` (89 lines)

A versatile button component with multiple variants, sizes, and states.

**Features:**
- Variants: `primary`, `secondary`, `tertiary`, `destructive`, `ghost`, `link`
- Sizes: `small`, `medium`, `large`, `icon`
- States: disabled, loading
- Icon support (left or right positioning)

**Usage:**
```tsx
import { Button } from "@/components/ui";
import { Send } from "lucide-react";

// Basic usage
<Button variant="primary">Click Me</Button>

// With icon
<Button 
  icon={<Send className="h-4 w-4" />} 
  iconPosition="left"
>
  Send Message
</Button>

// Loading state
<Button loading>Processing...</Button>

// Disabled
<Button disabled>Disabled</Button>
```

**Props:**
- `variant`: "primary" | "secondary" | "tertiary" | "destructive" | "ghost" | "link"
- `size`: "small" | "medium" | "large" | "icon"
- `loading`: boolean
- `disabled`: boolean
- `icon`: React.ReactNode
- `iconPosition`: "left" | "right"
- `asChild`: boolean (for composition with Slot)

---

### Input Component
**File:** `components/ui/input.tsx` (102 lines)

A flexible input component with various types and states, including password visibility toggle.

**Features:**
- Types: text, email, password (with show/hide toggle)
- States: error, disabled
- Label and helper text support
- Auto-generated IDs for accessibility

**Usage:**
```tsx
import { Input } from "@/components/ui";

// Basic text input
<Input 
  label="Name" 
  placeholder="Enter your name" 
/>

// Email input
<Input 
  type="email" 
  label="Email" 
  placeholder="you@example.com" 
/>

// Password with toggle
<Input 
  type="password" 
  label="Password" 
  placeholder="Enter password" 
/>

// With error
<Input 
  label="Username" 
  error="This field is required" 
/>

// With helper text
<Input 
  label="Bio" 
  helperText="Tell us about yourself" 
/>
```

**Props:**
- `label`: string
- `error`: string
- `helperText`: string
- `type`: "text" | "email" | "password" | etc.
- `disabled`: boolean
- All standard HTML input attributes

---

### Toast Component
**File:** `components/ui/toast.tsx` (85 lines)
**Provider:** `components/ui/toast-provider.tsx` (62 lines)

A notification toast system with multiple variants and auto-dismiss functionality.

**Features:**
- Variants: `success`, `error`, `warning`, `info`
- Auto-dismiss with configurable duration
- Close button
- Stacked notifications
- Icons for each variant

**Usage:**
```tsx
import { ToastProvider, useToast } from "@/components/ui";

// Wrap your app with ToastProvider
function App() {
  return (
    <ToastProvider>
      <YourApp />
    </ToastProvider>
  );
}

// Use in components
function MyComponent() {
  const { addToast } = useToast();

  const showSuccess = () => {
    addToast({
      variant: "success",
      title: "Success!",
      description: "Your action was completed.",
      duration: 5000, // optional, default 5000ms
    });
  };

  const showError = () => {
    addToast({
      variant: "error",
      title: "Error",
      description: "Something went wrong.",
    });
  };

  return (
    <div>
      <button onClick={showSuccess}>Show Success</button>
      <button onClick={showError}>Show Error</button>
    </div>
  );
}
```

**Toast Props:**
- `variant`: "success" | "error" | "warning" | "info"
- `title`: string
- `description`: string
- `duration`: number (milliseconds, 0 = no auto-dismiss)

---

### Checkbox Component
**File:** `components/ui/checkbox.tsx` (103 lines)

An accessible checkbox component with multiple states.

**Features:**
- States: checked, unchecked, indeterminate
- Error and disabled states
- Label support
- Helper text
- Fully accessible

**Usage:**
```tsx
import { Checkbox } from "@/components/ui";
import { useState } from "react";

function MyForm() {
  const [checked, setChecked] = useState(false);

  return (
    <div>
      {/* Basic checkbox */}
      <Checkbox 
        label="Accept terms" 
        checked={checked}
        onCheckedChange={setChecked}
      />

      {/* Indeterminate state */}
      <Checkbox 
        label="Select all" 
        indeterminate={true}
      />

      {/* With error */}
      <Checkbox 
        label="Required field" 
        error="You must accept to continue"
      />

      {/* With helper text */}
      <Checkbox 
        label="Subscribe" 
        helperText="Get weekly updates"
      />

      {/* Disabled */}
      <Checkbox 
        label="Disabled option" 
        disabled 
      />
    </div>
  );
}
```

**Props:**
- `label`: string
- `error`: string
- `helperText`: string
- `checked`: boolean | "indeterminate"
- `indeterminate`: boolean
- `disabled`: boolean
- `onCheckedChange`: (checked: boolean) => void
- All Radix Checkbox props

---

### Tooltip Component
**File:** `components/ui/tooltip.tsx` (79 lines)

A tooltip component for providing contextual information on hover or focus.

**Features:**
- Positioning: top, right, bottom, left
- Custom content support
- Hover and focus triggers
- Arrow indicator
- Configurable delay

**Usage:**
```tsx
import { Tooltip } from "@/components/ui";
import { Button } from "@/components/ui";

// Basic tooltip
<Tooltip content="This is a tooltip" side="top">
  <Button>Hover me</Button>
</Tooltip>

// Different positions
<Tooltip content="Tooltip on right" side="right">
  <Button>Right</Button>
</Tooltip>

// Custom content
<Tooltip 
  content={
    <div>
      <div className="font-semibold">Title</div>
      <div>Custom content here</div>
    </div>
  }
  side="bottom"
>
  <Button>Custom</Button>
</Tooltip>

// Without arrow
<Tooltip 
  content="No arrow" 
  showArrow={false}
>
  <Button>No Arrow</Button>
</Tooltip>

// Custom delay
<Tooltip 
  content="Quick tooltip" 
  delayDuration={0}
>
  <Button>Instant</Button>
</Tooltip>
```

**Props:**
- `content`: React.ReactNode
- `side`: "top" | "right" | "bottom" | "left"
- `delayDuration`: number (milliseconds)
- `showArrow`: boolean

---

## 🚀 Installation & Setup

All dependencies are already installed. The components are located in:
```
frontend/
├── components/
│   └── ui/
│       ├── button.tsx
│       ├── input.tsx
│       ├── toast.tsx
│       ├── toast-provider.tsx
│       ├── checkbox.tsx
│       ├── tooltip.tsx
│       └── index.ts
└── lib/
    └── utils.ts
```

## 📝 Usage

Import components from the central export:
```tsx
import { 
  Button, 
  Input, 
  Checkbox, 
  Tooltip,
  ToastProvider,
  useToast 
} from "@/components/ui";
```

## 🎯 Demo

View all components in action:
```bash
cd frontend
pnpm dev
```

Visit: `http://localhost:3000/components-demo`

## ♿ Accessibility

All components follow accessibility best practices:
- ✅ Proper ARIA labels and roles
- ✅ Keyboard navigation support
- ✅ Focus management
- ✅ Screen reader compatibility
- ✅ Error announcements
- ✅ Semantic HTML

## 🎨 Customization

Components use Tailwind CSS and can be customized via:
1. **CSS Variables** in `app/globals.css`
2. **Tailwind Classes** via the `className` prop
3. **CVA Variants** in component files

Example theme colors (dark mode):
```css
--color-primary: oklch(0.65 0.15 200);      /* Cyan/Teal */
--color-secondary: oklch(0.25 0.04 240);    /* Dark Blue */
--color-destructive: oklch(0.50 0.20 25);   /* Red */
--color-success: oklch(0.50 0.15 150);      /* Green */
--color-warning: oklch(0.70 0.15 60);       /* Orange */
--color-info: oklch(0.60 0.15 230);         /* Blue */
```

## 📊 Component Metrics

| Component | Lines | Features | Accessibility |
|-----------|-------|----------|---------------|
| Button    | 89    | 6 variants, 4 sizes, loading, icons | ✅ Full |
| Input     | 102   | 3 types, password toggle, errors | ✅ Full |
| Toast     | 85    | 4 variants, auto-dismiss, close | ✅ Full |
| Checkbox  | 103   | 3 states, errors, labels | ✅ Full |
| Tooltip   | 79    | 4 positions, custom content | ✅ Full |

## 🔧 Technical Details

**Built with:**
- Next.js 16.1.3
- React 19.2.3
- TypeScript 5.9.3
- Tailwind CSS 4.1.18
- Radix UI primitives
- class-variance-authority
- lucide-react icons

## 📖 Additional Resources

- [shadcn/ui Documentation](https://ui.shadcn.com/)
- [Radix UI Documentation](https://www.radix-ui.com/)
- [Figma Design](https://www.figma.com/design/QMi4SBZnJ7HkXJxrW8pcNC/PredFI?node-id=1503-9561&t=Djsvmj0JhCNCkhTm-0)

## ✅ Acceptance Criteria Met

- ✅ shadcn/ui installed and configured
- ✅ Button component with all variants and states
- ✅ Input component with all types and states
- ✅ Toast component with all variants
- ✅ Checkbox component with all states
- ✅ Tooltip component with positioning options
- ✅ All components match design specifications
- ✅ TypeScript types for all props
- ✅ Full accessibility support
- ✅ Each component under 150 lines
- ✅ Components exported from central location
- ✅ Clean, optimized code
- ✅ Documentation provided
- ✅ Demo page created
- ✅ No console errors or warnings
- ✅ All components reusable
