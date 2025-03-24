"use client";

import { forwardRef, type InputHTMLAttributes } from "react";
import type { FieldError } from "react-hook-form";

interface FormInputProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: FieldError;
}

const FormInput = forwardRef<HTMLInputElement, FormInputProps>(
  ({ label, error, className, id, ...props }, ref) => {
    return (
      <div className={`flex gap-1 flex-col text-base ${className || ""}`}>
        <label htmlFor={id || props.name}>{label}</label>
        <input
          ref={ref}
          id={id || props.name}
          className={`border-[#373737] bg-inherit border rounded-[8px] h-[45px] px-4 outline-none w-full focus:border-gray-500 transition-colors ${
            error ? "border-red-500" : ""
          }`}
          {...props}
        />
        {error && <p className="text-red-500 text-xs mt-1">{error.message}</p>}
      </div>
    );
  }
);

FormInput.displayName = "FormInput";

export default FormInput;
