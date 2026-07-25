"use client";

import { useState } from "react";
import { Send, Heart, Download } from "lucide-react";
import {
  Button,
  Input,
  SearchBar,
  Checkbox,
  Tooltip,
  CopyButton,
  useToastActions,
} from "@/components/ui";

function ComponentShowcase() {
  const [checkedState, setCheckedState] = useState(false);
  const [indeterminateState, setIndeterminateState] = useState(true);
  const [searchResult, setSearchResult] = useState("");
  const { addToast } = useToastActions();

  const handleToastSuccess = () => {
    addToast({
      variant: "success",
      title: "Success!",
      description: "Your action was completed successfully.",
      duration: 5000,
    });
  };

  const handleToastError = () => {
    addToast({
      variant: "error",
      title: "Error occurred",
      description: "Something went wrong. Please try again.",
      duration: 5000,
    });
  };

  const handleToastWarning = () => {
    addToast({
      variant: "warning",
      title: "Warning",
      description: "Please review your information before proceeding.",
      duration: 5000,
    });
  };

  const handleToastInfo = () => {
    addToast({
      variant: "info",
      title: "Information",
      description: "Here is some helpful information for you.",
      duration: 5000,
    });
  };

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-6xl mx-auto space-y-12">
        <div>
          <h1 className="text-4xl font-bold mb-2">UI Components Library</h1>
          <p className="text-muted-foreground">
            PrediFi reusable components built with shadcn/ui
          </p>
        </div>

        {/* Button Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Button Component</h2>
            <p className="text-muted-foreground mb-4">
              Various button variants and states
            </p>
          </div>

          <div className="space-y-4">
            <div className="flex flex-wrap gap-4 items-center">
              <Button variant="primary">Primary Button</Button>
              <Button variant="secondary">Secondary Button</Button>
              <Button variant="tertiary">Tertiary Button</Button>
              <Button variant="destructive">Destructive</Button>
              <Button variant="ghost">Ghost</Button>
              <Button variant="link">Link Button</Button>
            </div>

            <div className="flex flex-wrap gap-4 items-center">
              <Button size="small">Small</Button>
              <Button size="medium">Medium</Button>
              <Button size="large">Large</Button>
            </div>

            <div className="flex flex-wrap gap-4 items-center">
              <Button disabled>Disabled</Button>
              <Button loading>Loading</Button>
              <Button icon={<Send className="h-4 w-4" />} iconPosition="left">
                With Icon
              </Button>
              <Button icon={<Heart className="h-4 w-4" />} iconPosition="right">
                Icon Right
              </Button>
              <Button size="icon">
                <Download className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </section>

        {/* Input Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Input Component</h2>
            <p className="text-muted-foreground mb-4">
              Various input types and states
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-3xl">
            <Input label="Text Input" placeholder="Enter your name" />
            <Input
              type="email"
              label="Email Input"
              placeholder="you@example.com"
            />
            <Input
              type="password"
              label="Password Input"
              placeholder="Enter password"
            />
            <Input
              label="Input with Error"
              error="This field is required"
              placeholder="Required field"
            />
            <Input label="Disabled Input" disabled placeholder="Disabled" />
            <Input
              label="Input with Helper"
              helperText="This is a helpful message"
              placeholder="Helper text"
            />
          </div>
        </section>

        {/* SearchBar Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">SearchBar Component</h2>
            <p className="text-muted-foreground mb-4">
              Debounced search input — <code>onSearch</code> fires only after
              the user stops typing (default 300 ms delay).
            </p>
          </div>

          <div className="space-y-6 max-w-xl">
            {/* Default (300 ms debounce) */}
            <div className="space-y-2">
              <p className="text-sm font-medium">Default (300 ms debounce)</p>
              <SearchBar
                placeholder="Search pools..."
                onSearch={(query) => setSearchResult(query)}
              />
            </div>

            {/* Custom delay */}
            <div className="space-y-2">
              <p className="text-sm font-medium">Custom delay (500 ms)</p>
              <SearchBar
                placeholder="Search predictions..."
                debounceDelay={500}
                onSearch={(query) => setSearchResult(query)}
              />
            </div>

            {/* Disabled state */}
            <div className="space-y-2">
              <p className="text-sm font-medium">Disabled</p>
              <SearchBar
                placeholder="Search disabled..."
                disabled
                onSearch={() => {}}
              />
            </div>

            {/* Live feedback */}
            {searchResult && (
              <p className="text-sm text-muted-foreground">
                Last debounced query:{" "}
                <span className="text-foreground font-medium">
                  &quot;{searchResult}&quot;
                </span>
              </p>
            )}
          </div>
        </section>

        {/* Toast Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Toast Component</h2>
            <p className="text-muted-foreground mb-4">
              Notification toasts with slide-in/out animations, progress bar,
              pause-on-hover, optional action button, and persistent mode.
            </p>
          </div>

          <div className="space-y-6">
            {/* Variants */}
            <div>
              <p className="text-sm font-medium mb-3">Variants</p>
              <div className="flex flex-wrap gap-3">
                <Button onClick={handleToastSuccess} variant="primary" size="small">
                  Success
                </Button>
                <Button onClick={handleToastError} variant="destructive" size="small">
                  Error
                </Button>
                <Button onClick={handleToastWarning} variant="secondary" size="small">
                  Warning
                </Button>
                <Button onClick={handleToastInfo} variant="tertiary" size="small">
                  Info
                </Button>
              </div>
            </div>

            {/* With action button */}
            <div>
              <p className="text-sm font-medium mb-3">With action button</p>
              <div className="flex flex-wrap gap-3">
                <Button
                  size="small"
                  variant="secondary"
                  onClick={() =>
                    addToast({
                      variant: "success",
                      title: "Pool created",
                      description: "Your prediction pool is live.",
                      action: { label: "View pool", onClick: () => {} },
                      duration: 8000,
                    })
                  }
                >
                  With &quot;View pool&quot; action
                </Button>
                <Button
                  size="small"
                  variant="secondary"
                  onClick={() =>
                    addToast({
                      variant: "warning",
                      title: "Transaction pending",
                      description: "Your stake is being processed.",
                      action: { label: "Undo", onClick: () => {} },
                      duration: 8000,
                    })
                  }
                >
                  With &quot;Undo&quot; action
                </Button>
              </div>
            </div>

            {/* Persistent (no auto-dismiss) */}
            <div>
              <p className="text-sm font-medium mb-3">Persistent (manual dismiss only)</p>
              <Button
                size="small"
                variant="tertiary"
                onClick={() =>
                  addToast({
                    variant: "error",
                    title: "Wallet disconnected",
                    description: "Reconnect your wallet to continue.",
                    persistent: true,
                  })
                }
              >
                Persistent error toast
              </Button>
            </div>
          </div>
        </section>

        {/* CopyButton Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">CopyButton Component</h2>
            <p className="text-muted-foreground mb-4">
              Copy-to-clipboard button with toast feedback and icon swap
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-8">
            <div className="space-y-1">
              <p className="text-sm font-medium text-muted-foreground">Size xs</p>
              <div className="flex items-center gap-2 font-mono text-sm">
                0xA3B2...C19F
                <CopyButton
                  text="0xA3B2...C19F"
                  size="xs"
                  aria-label="Copy address"
                />
              </div>
            </div>

            <div className="space-y-1">
              <p className="text-sm font-medium text-muted-foreground">Size sm (default)</p>
              <div className="flex items-center gap-2 font-mono text-sm">
                REF-001
                <CopyButton
                  text="REF-001"
                  size="sm"
                  copyOptions={{ successDescription: "REF-001 copied to clipboard" }}
                />
              </div>
            </div>

            <div className="space-y-1">
              <p className="text-sm font-medium text-muted-foreground">Size md</p>
              <div className="flex items-center gap-2 font-mono text-sm">
                predifi.app/ref/abc123
                <CopyButton
                  text="https://predifi.app/ref/abc123"
                  size="md"
                  copyOptions={{ successTitle: "Link copied!", successDescription: "Share your referral link." }}
                />
              </div>
            </div>
          </div>
        </section>

        {/* Checkbox Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Checkbox Component</h2>
            <p className="text-muted-foreground mb-4">
              Checkboxes with various states
            </p>
          </div>

          <div className="space-y-4 max-w-md">
            <Checkbox
              label="Basic Checkbox"
              checked={checkedState}
              onCheckedChange={(checked) =>
                setCheckedState(checked as boolean)
              }
            />
            <Checkbox
              label="Indeterminate Checkbox"
              indeterminate={indeterminateState}
              checked={indeterminateState}
              onCheckedChange={(checked) =>
                setIndeterminateState(checked as boolean)
              }
            />
            <Checkbox
              label="Checkbox with Helper"
              helperText="This is a helpful message"
            />
            <Checkbox
              label="Checkbox with Error"
              error="This field is required"
            />
            <Checkbox label="Disabled Checkbox" disabled />
            <Checkbox label="Disabled Checked" disabled checked />
          </div>
        </section>

        {/* Tooltip Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Tooltip Component</h2>
            <p className="text-muted-foreground mb-4">
              Tooltips with different positions
            </p>
          </div>

          <div className="flex flex-wrap gap-8">
            <Tooltip content="Tooltip on top" side="top">
              <Button variant="secondary">Top Tooltip</Button>
            </Tooltip>
            <Tooltip content="Tooltip on right" side="right">
              <Button variant="secondary">Right Tooltip</Button>
            </Tooltip>
            <Tooltip content="Tooltip on bottom" side="bottom">
              <Button variant="secondary">Bottom Tooltip</Button>
            </Tooltip>
            <Tooltip content="Tooltip on left" side="left">
              <Button variant="secondary">Left Tooltip</Button>
            </Tooltip>
            <Tooltip
              content={
                <div>
                  <div className="font-semibold">Custom Content</div>
                  <div>This tooltip has custom content</div>
                </div>
              }
              side="top"
            >
              <Button variant="primary">Custom Tooltip</Button>
            </Tooltip>
          </div>
        </section>
      </div>
    </div>
  );
}

export default function ComponentsDemo() {
  return <ComponentShowcase />;
}
