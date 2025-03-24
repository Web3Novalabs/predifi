"use client";

import { forwardRef, type TextareaHTMLAttributes } from "react";
import type { FieldError } from "react-hook-form";

interface FormTextareaProps
  extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label: string;
  error?: FieldError;
}

const FormTextarea = forwardRef<HTMLTextAreaElement, FormTextareaProps>(
  ({ label, error, className, id, ...props }, ref) => {
    return (
      <div className="flex gap-1 flex-col text-base w-full">
        <label htmlFor={id || props.name}>{label}</label>
        <textarea
          ref={ref}
          id={id || props.name}
          className={`border-[#373737] bg-inherit border rounded-[8px] h-[140px] w-full px-4 py-3 outline-none resize-none focus:border-gray-500 transition-colors ${
            error ? "border-red-500" : ""
          } ${className || ""}`}
          {...props}
        />
        {error && <p className="text-red-500 text-xs mt-1">{error.message}</p>}
      </div>
    );
  }
);

FormTextarea.displayName = "FormTextarea";

export default FormTextarea;
