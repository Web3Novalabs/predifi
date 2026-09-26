# UI Components

A collection of reusable, accessible React components for the Predifi application.

## Components

| Component | Purpose | Main Props |
|-----------|---------|------------|
| **Button** | Interactive button with variants (primary, secondary, tertiary, destructive, ghost, link) and sizes | `variant`, `size`, `asChild`, `loading`, `icon`, `iconPosition`, `disabled` |
| **Input** | Text input with optional label, error, helper text, and password visibility toggle | `label`, `error`, `helperText`, `type`, `disabled` |
| **SearchBar** | Search input with debounced callback for API calls | `onSearch`, `placeholder`, `debounceDelay`, `value`, `onChange`, `disabled`, `aria-label` |
| **Skeleton** | Layout-preserving loading placeholder (pulse or shimmer animation) | `variant`, `className`, `style` |
| **SkeletonText** | Multi-line text placeholder | `lines`, `lastLineWidth`, `variant`, `className` |
| **SkeletonCircle** | Circular placeholder for avatars/icons | `className`, `style`, `variant` |
| **SkeletonScreen** | Accessible wrapper for groups of skeletons with screen reader announcement | `children`, `className`, `label` |
| **PoolCardSkeleton** | Skeleton matching a pool card layout | `className` |
| **PoolListSkeleton** | Grid of pool card placeholders | `count`, `className` |
| **PoolDetailSkeleton** | Skeleton matching pool detail page layout | `className` |
| **PredictionTableSkeleton** | Table row placeholders | `rows`, `className` |
| **StatsSkeleton** | Dashboard stat tile placeholders | `count`, `className` |
| **SkipLink** | "Skip to main content" bypass link for keyboard users | `targetId`, `children`, `className` |
| **VisuallyHidden** | Content visible to screen readers only, or visible on focus | `children`, `as`, `className`, `focusable` |
| **LiveRegionProvider** | Context provider for ARIA live regions for screen reader announcements | `children` |
| **useAnnounce** | Hook to announce messages to screen readers | - |
| **Checkbox** | Accessible checkbox with label, error, helper text, and indeterminate state | `label`, `error`, `helperText`, `indeterminate`, `disabled`, `checked` |
| **Tooltip** | Tooltip wrapper with provider, trigger, content, arrow, and positioning | `content`, `children`, `side`, `delayDuration`, `showArrow` |
| **TooltipProvider** | Context provider for tooltip configuration | `children`, `delayDuration` |
| **TooltipRoot** | Root tooltip component from Radix UI | - |
| **TooltipTrigger** | Trigger element for tooltip | - |
| **TooltipContent** | Content element for tooltip | `className`, `sideOffset` |
| **TooltipArrow** | Arrow element for tooltip | `className` |
| **SocialIcon** | Social media icon rendered from SVG sprite | `id`, `label`, `className` |
| **StakeInput** | Numeric input for stake amounts with auto-sanitization and token suffix | `label`, `error`, `helperText`, `token`, `value`, `onChange`, `disabled`, `placeholder` |
| **Card** | Container component with header, footer, title, description, and content slots | Inherits `HTMLAttributes<HTMLDivElement>` |
| **CardHeader** | Card header section | Inherits `HTMLAttributes<HTMLDivElement>` |
| **CardFooter** | Card footer section | Inherits `HTMLAttributes<HTMLDivElement>` |
| **CardTitle** | Card title element | Inherits `HTMLAttributes<HTMLHeadingElement>` |
| **CardDescription** | Card description element | Inherits `HTMLAttributes<HTMLParagraphElement>` |
| **CardContent** | Card content section | Inherits `HTMLAttributes<HTMLDivElement>` |
| **Toast** | Notification toast with auto-dismiss, progress bar, and optional action button | `variant`, `id`, `title`, `description`, `action`, `onClose`, `duration`, `persistent` |
| **ToastProvider** | Context provider for toast management with configurable position and max count | `children`, `position`, `maxToasts` |
| **useToast** | Hook to access toast state and actions | - |
| **useToastActions** | Hook to access toast actions (addToast, removeToast) without re-renders | - |
| **PayoutEstimator** | UI to estimate prediction payouts based on stake and odds | `token`, `className` |
| **TransactionProgress** | Visual progress indicator for transaction status | `status`, `txHash`, `errorMessage`, `className` |
| **ShareButton** | Share button with dropdown for social networks | `url`, `title`, `text`, `className`, `networks` |
| **CopyButton** | Icon-only button to copy text to clipboard with visual feedback | `text`, `size`, `copyOptions`, `disabled`, `aria-label` |
| **WhitelistErrorBanner** | Error banner for private pool whitelist issues | `className`, `message`, `onDismiss` |
| **SupportedTokensPicker** | Dropdown to select supported tokens | `value`, `onChange`, `tokens`, `className`, `disabled` |
| **ThemeToggle** | Theme switcher (dark/light/system) | `className` |
| **OddsCalculator** | Calculator to determine odds based on probability split | `token`, `className` |

## Export Structure

Components are exported from `index.ts`. For tooltip components, you can import the wrapper or the underlying Radix primitives:

```tsx
// Wrapper (recommended)
import { Tooltip, TooltipProvider } from "@/components/ui/tooltip";

// Or import primitives directly
import { TooltipProvider, TooltipRoot, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
```
