"use client";

import { useState } from "react";
import { Send, Heart, Download } from "lucide-react";
import {
  Button,
  Input,
  Checkbox,
  Tooltip,
  ToastProvider,
  useToast,
} from "@/components/ui";

function ComponentShowcase() {
  const [checkedState, setCheckedState] = useState(false);
  const [indeterminateState, setIndeterminateState] = useState(true);
  const { addToast } = useToast();

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

        {/* Toast Component */}
        <section className="space-y-6">
          <div>
            <h2 className="text-2xl font-semibold mb-2">Toast Component</h2>
            <p className="text-muted-foreground mb-4">
              Notification toasts with different variants
            </p>
          </div>

          <div className="flex flex-wrap gap-4">
            <Button onClick={handleToastSuccess} variant="primary">
              Show Success Toast
            </Button>
            <Button onClick={handleToastError} variant="destructive">
              Show Error Toast
            </Button>
            <Button onClick={handleToastWarning} variant="secondary">
              Show Warning Toast
            </Button>
            <Button onClick={handleToastInfo} variant="tertiary">
              Show Info Toast
            </Button>
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
  return (
    <ToastProvider>
      <ComponentShowcase />
    </ToastProvider>
  );
}
