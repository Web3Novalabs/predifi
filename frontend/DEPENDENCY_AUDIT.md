# Dependency Audit Report

This document reports the findings of the NPM dependency audit conducted as part of issue #955 ("Audit package.json and remove unused libraries").

## Summary
The audit has concluded that **all dependencies** listed in `frontend/package.json` are actively used within the codebase. No unused libraries were identified for removal.

## Audited Dependencies

### Production Dependencies
| Library | Usage | Reference Files |
| :--- | :--- | :--- |
| `@radix-ui/react-checkbox` | Checkbox UI primitive | `components/ui/checkbox.tsx` |
| `@radix-ui/react-slot` | Polymorphic component support (asChild) | `components/ui/button.tsx` |
| `@radix-ui/react-tooltip` | Tooltip UI primitive | `components/ui/tooltip.tsx` |
| `class-variance-authority` | CSS-in-JS variant management | `ui/button.tsx`, `ui/toast.tsx` |
| `clsx` | Conditional class joining | `lib/utils.ts` |
| `lucide-react` | Icon library (Menu, X, Loader, etc.) | Widespread (Main Nav, Buttons, Cards) |
| `next` | Core Framework | `app/`, `next.config.ts` |
| `react` | Core UI Library | Widespread |
| `react-dom` | Core UI Library (Web Support) | Widespread |
| `recharts` | Data visualization charts | `components/dashboard/StakedChart.tsx` |
| `tailwind-merge` | Smart Tailwind class conflict resolution | `lib/utils.ts` |

### Development Dependencies
| Library | usage |
| :--- | :--- |
| `autoprefixer` | PostCSS plugin for browser prefixing in `postcss.config.mjs` |
| `eslint` & `eslint-config-next` | Linting configuration in `eslint.config.mjs` |
| `postcss` | CSS transformation engine |
| `tailwindcss` | Utility-first CSS framework |
| `typescript` | Static typing support |
| `@types/*` | Type definitions for Node, React, and DOM |

## Audit Methodology
1.  **Code Scoping**: Scanned all `.tsx`, `.ts`, and `.mjs` files in the `frontend/app` and `frontend/components` directories.
2.  **Package.json Analysis**: Iterated through every entry in `dependencies` and `devDependencies`.
3.  **Import Verification**: Verified each package against `import` and `require` statements using automated grep searches.
4.  **UI Component Cross-Reference**: Ensured that UI primitives (Radix UI) are mapped to exported components in `components/ui/index.ts`.

## Conclusion
The project's dependency list is highly optimized. Every library serves a specific purpose in the current architecture, from the core framework to the specialized visualization and styling utilities.

*Audit Date: June 2, 2026*
