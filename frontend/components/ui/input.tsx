import React from "react";

export interface InputProps {
  label: string;
  type?: string;
  name?: string;
  id?: string;
  placeholder?: string;
  value?: string;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  error?: string;
  helperText?: string;
  disabled?: boolean;
}

const Input: React.FC<InputProps> = ({
  label,
  type = "text",
  name,
  id,
  placeholder,
  value,
  onChange,
  error,
  helperText,
  disabled,
}) => {
  return (
    <div className="flex flex-col h-auto min-h-[72px] cols-span-1 gap-3">
      <label htmlFor={id || name} className="text-[#8F8F8F]">
        {label}
      </label>

      <input
        type={type}
        id={id || name}
        name={name}
        placeholder={placeholder}
        value={value}
        onChange={onChange}
        disabled={disabled}
        className={`bg-transparent outline-none border-[#373737] border rounded-[8px] text-[16px] font-[400] text-[#CCCCCC] p-3 min-h-[43px]
        ${error ? "border-red-500" : "border-[#373737]"}
        ${disabled ? "opacity-50 cursor-not-allowed" : ""}`}
        style={type === "date" ? { WebkitAppearance: "none" } : {}}
      />

      {error && <p className="text-red-500 text-sm">{error}</p>}
      {helperText && !error && <p className="text-[#8F8F8F] text-sm">{helperText}</p>}
    </div>
  );
};

export default Input;
