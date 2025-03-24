"use client";

import { type Control, Controller } from "react-hook-form";
import FormInput from "./ui/form-input";
import type { creatorInputs } from "@/type/type";

interface BetAmountSettingsProps {
  control: Control<creatorInputs>;
}

const BetAmountSettings = ({ control }: BetAmountSettingsProps) => {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
      <Controller
        name="minBetAmount"
        control={control}
        rules={{
          required: "Min bet amount is required",
          min: { value: 0, message: "Min bet amount must be positive" },
        }}
        render={({ field, fieldState }) => (
          <FormInput
            label="Min bet amount"
            type="number"
            // placeholder="Pool min stake amount"
            error={fieldState.error}
            {...field}
          />
        )}
      />
      <Controller
        name="maxBetAmount"
        control={control}
        rules={{
          required: "Max bet amount is required",
          min: { value: 0, message: "Max bet amount must be positive" },
          validate: (value, formValues) =>
            Number(value) > Number(formValues.minBetAmount) ||
            "Max bet must be greater than min bet",
        }}
        render={({ field, fieldState }) => (
          <FormInput
            label="Max bet amount"
            type="number"
            // placeholder="Pool max stake amount"
            error={fieldState.error}
            {...field}
          />
        )}
      />
      <Controller
        name="creatorsFee"
        control={control}
        rules={{
          required: "Creator's fee is required",
          min: { value: 0, message: "Fee must be at least 0%" },
          max: { value: 5, message: "Fee cannot exceed 5%" },
        }}
        render={({ field, fieldState }) => (
          <FormInput
            label="Creator's fee"
            type="number"
            // placeholder="Max is 5%"
            error={fieldState.error}
            {...field}
          />
        )}
      />
    </div>
  );
};

export default BetAmountSettings;
