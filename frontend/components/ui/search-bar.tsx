"use client";

import * as React from "react";
import { Search, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useDebounce } from "@/lib/hooks/useDebounce";

export interface SearchBarProps {
  /**
   * Callback fired with the debounced search value.
   * This is the primary handler to use for triggering API calls.
   */
  onSearch: (value: string) => void;
  /** Placeholder text shown inside the input. */
  placeholder?: string;
  /** Debounce delay in milliseconds before `onSearch` is called. Defaults to 300ms. */
  debounceDelay?: number;
  /** Optional controlled value. When provided the component behaves as a controlled input. */
  value?: string;
  /** Optional callback fired on every keystroke (before debouncing). */
  onChange?: (value: string) => void;
  /** Additional class names applied to the outer wrapper. */
  className?: string;
  /** Disables the input. */
  disabled?: boolean;
  /** Accessible label for the search input (used by screen readers). */
  "aria-label"?: string;
}

/**
 * SearchBar
 *
 * A search input with built-in debouncing to reduce the number of API calls
 * triggered while the user is typing. The `onSearch` callback is only invoked
 * after the user has stopped typing for `debounceDelay` milliseconds.
 *
 * @example
 * // Basic usage — fires API call 400 ms after the user stops typing
 * <SearchBar
 *   placeholder="Search pools..."
 *   debounceDelay={400}
 *   onSearch={(query) => fetchPools(query)}
 * />
 */
export function SearchBar({
  onSearch,
  placeholder = "Search...",
  debounceDelay = 300,
  value: controlledValue,
  onChange,
  className,
  disabled = false,
  "aria-label": ariaLabel = "Search",
}: SearchBarProps) {
  // Internal state for the raw (non-debounced) input value.
  // When `controlledValue` is provided we use it as the source of truth.
  const [inputValue, setInputValue] = React.useState<string>(
    controlledValue ?? "",
  );
  const isControlled = controlledValue !== undefined;
  const currentValue = isControlled ? controlledValue : inputValue;

  // The debounced value — only updates after the user pauses typing.
  const debouncedValue = useDebounce(currentValue, debounceDelay);

  // Fire `onSearch` whenever the debounced value changes.
  React.useEffect(() => {
    onSearch(debouncedValue);
  }, [debouncedValue, onSearch]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setInputValue(newValue);
    onChange?.(newValue);
  };

  const handleClear = () => {
    if (!isControlled) {
      setInputValue("");
    }
    onChange?.("");
  };

  return (
    <div className={cn("relative flex items-center w-full", className)}>
      {/* Search icon */}
      <Search
        className="absolute left-3 h-4 w-4 text-muted-foreground pointer-events-none"
        aria-hidden="true"
      />

      <input
        type="search"
        role="searchbox"
        aria-label={ariaLabel}
        value={currentValue}
        onChange={handleChange}
        placeholder={placeholder}
        disabled={disabled}
        autoComplete="off"
        className={cn(
          // Base styles
          "flex h-10 w-full rounded-md border border-input bg-transparent",
          // Padding — left leaves room for the search icon, right for the clear button
          "pl-9 pr-9 py-2 text-sm",
          // Ring / focus styles
          "ring-offset-background",
          "placeholder:text-muted-foreground",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          // Disabled state
          "disabled:cursor-not-allowed disabled:opacity-50",
          // Remove the browser's native search cancel button so we can render our own
          "[&::-webkit-search-cancel-button]:appearance-none",
        )}
      />

      {/* Clear button — only visible when there is text */}
      {inputValue && !disabled && (
        <button
          type="button"
          onClick={handleClear}
          aria-label="Clear search"
          className="absolute right-3 text-muted-foreground hover:text-foreground transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
