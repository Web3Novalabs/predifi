"use client";

import { type Control, Controller } from "react-hook-form";
import FormInput from "../ui/form-input";
import FormSelect from "../ui/form-select";
import type { creatorInputs } from "@/type/type";

interface PoolOptionsProps {
  control: Control<creatorInputs>;
}
const categoryOptions = [
  { value: "", label: "" }, 
  { value: 1, label: "Sport" },
  { value: 2, label: "Politics" },
  { value: 3, label: "Entertainment" },
  { value: 4, label: "Crypto" },
  { value: 5, label: "Others" },
];

const PoolOptions = ({ control }: PoolOptionsProps) => {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
      <Controller
        name="optionOne"
        control={control}
        rules={{ required: "Option 1 is required" }}
        render={({ field, fieldState }) => (
          <FormInput
            label="Option 1"
            // placeholder="Enter option 1"
            error={fieldState.error}
            {...field}
          />
        )}
      />
      <Controller
        name="optionTwo"
        control={control}
        rules={{ required: "Option 2 is required" }}
        render={({ field, fieldState }) => (
          <FormInput
            label="Option 2"
            // placeholder="Enter option 2"
            error={fieldState.error}
            {...field}
          />
        )}
      />
      <Controller
        name="category"
        control={control}
        rules={{ required: "Category is required" }}
        render={({ field, fieldState }) => (
          <FormSelect
            label="Category"
            options={categoryOptions}
            error={fieldState.error}
            {...field}
          />
        )}
      />
    </div>
  );
};

export default PoolOptions;
