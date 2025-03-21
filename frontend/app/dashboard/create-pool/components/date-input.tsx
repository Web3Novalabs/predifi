"use client";

import { type Control, Controller } from "react-hook-form";
import Datetime from "react-datetime";
import "react-datetime/css/react-datetime.css";
import type { creatorInputs } from "@/type/type";
import moment from "moment";

interface DateInputProps {
  name: string;
  label: string;
  control: Control<creatorInputs>;
}

export const DateInput = ({ name, label, control }: DateInputProps) => {
  return (
    <div className="flex gap-1 flex-col text-base w-full mb-2">
      <label htmlFor={name}>{label}</label>
      <Controller
        name={name as keyof creatorInputs}
        control={control}
        rules={{ required: `${label} is required` }}
        render={({ field, fieldState }) => (
          <>
            <Datetime
              inputProps={{
                id: name,
                className: `border-[#373737] bg-inherit border rounded-[8px] h-[45px] w-full px-4 outline-none ${
                  fieldState.error ? "border-red-500" : ""
                }`,
                // placeholder: `Select ${label.toLowerCase()}`
              }}
              onChange={(date) => {
                if (date && typeof date !== "string") {
                  field.onChange(date.toDate());
                }
              }}
              // Convert to moment object if it's a Date or timestamp
              value={field.value ? moment(field.value) : undefined}
              closeOnSelect
              timeFormat="HH:mm"
            />
            {fieldState.error && (
              <p className="text-red-500 text-xs mt-1">
                {fieldState.error.message}
              </p>
            )}
          </>
        )}
      />
    </div>
  );
};
