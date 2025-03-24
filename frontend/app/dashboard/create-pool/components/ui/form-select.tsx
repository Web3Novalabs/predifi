"use client";

import { forwardRef, type SelectHTMLAttributes } from "react";
import type { FieldError } from "react-hook-form";

interface FormSelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label: string;
  options: { value: string | number; label: string }[];
  error?: FieldError;
}

const FormSelect = forwardRef<HTMLSelectElement, FormSelectProps>(
  ({ label, options, error, className, id, ...props }, ref) => {
    return (
      <div className={`flex gap-1 flex-col text-base ${className || ""}`}>
        <label htmlFor={id || props.name}>{label}</label>
        <select
          ref={ref}
          id={id || props.name}
          className={`border-[#373737] bg-inherit border rounded-[8px] h-[45px] px-2 outline-none w-full focus:border-gray-500 transition-colors ${
            error ? "border-red-500" : ""
          }`}
          {...props}
        >
          {options.map((option) => (
            <option
              key={option.value}
              value={option.value}
              className="bg-[#373737]"
            >
              {option.label}
            </option>
          ))}
        </select>
        {error && <p className="text-red-500 text-xs mt-1">{error.message}</p>}
      </div>
    );
  }
);

FormSelect.displayName = "FormSelect";

export default FormSelect;
