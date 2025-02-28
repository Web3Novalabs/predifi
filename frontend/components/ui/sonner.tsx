"use client";

import { useTheme } from "next-themes";
import { Toaster as Sonner, toast, ToastClassnames } from "sonner";

type ToasterProps = React.ComponentProps<typeof Sonner>;

type ToastOptions = {
  className?: string;
  closeButton?: boolean;
  descriptionClassName?: string;
  style?: React.CSSProperties;
  cancelButtonStyle?: React.CSSProperties;
  actionButtonStyle?: React.CSSProperties;
  duration?: number;
  unstyled?: boolean;
  classNames?: ToastClassnames;
  closeButtonAriaLabel?: string;
};


const Toaster = ({ ...props }: ToasterProps) => {
  const { theme = "system" } = useTheme();

  return (
    <Sonner
      theme={theme as ToasterProps["theme"]}
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            "group toast group-[.toaster]:bg-background group-[.toaster]:text-foreground group-[.toaster]:border-border group-[.toaster]:shadow-lg",
          description: "group-[.toast]:text-muted-foreground",
          actionButton:
            "group-[.toast]:bg-primary group-[.toast]:text-primary-foreground",
          cancelButton:
            "group-[.toast]:bg-muted group-[.toast]:text-muted-foreground",
        },
      }}
      {...props}
    />
  );
};

Toaster.success = (message: string, options?: ToastOptions) => {
  toast.success(message, options);
};

Toaster.error = (message: string, options?: ToastOptions) => {
  toast.error(message, options);
};

Toaster.info = (message: string, options?: ToastOptions) => {
  toast.info(message, options);
};

Toaster.warning = (message: string, options?: ToastOptions) => {
  toast.warning(message, options);
};

export { Toaster };
