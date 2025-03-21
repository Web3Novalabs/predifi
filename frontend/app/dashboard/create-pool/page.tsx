"use client";

import { Controller, type SubmitHandler, useForm } from "react-hook-form";
import "react-datetime/css/react-datetime.css";
import { useState } from "react";
import type { creatorInputs } from "@/type/type";
import { DateInput } from "./components/pool/date-input";
import { useAccount } from "@starknet-react/core";
import { useCustomReadContract } from "@/hooks/useContractFetch";
import { abi } from "@/lib/abi";
import { useCreatePool } from "@/hooks/use-create-pool";
import CreatePoolModal from "./components/create-pool-modal";
import { prediFiContract, predifiContractAddress, sendFn } from "@/lib/send-fn";
import Loading from "@/components/loading-spinner";
import FormInput from "./components/ui/form-input";
import FormSelect from "./components/ui/form-select";
import FormTextarea from "./components/ui/form-textarea";
import ImageUpload from "./components/pool/image-upload";
import PoolOptions from "./components/pool/pool-options";
import BetAmountSettings from "./components/pool/bet-amount-settings";

function CreatePoolForm() {
  const [image, setImage] = useState<null | string>(null);
  const [startDate, setStartDate] = useState<number>();
  const [lockDate, setLockDate] = useState<number>();
  const [endDate, setEndDate] = useState<number>();
  const [isOpen, setIsOpen] = useState<boolean>(false);

  const { handleSubmit, watch, control } = useForm<creatorInputs>({
    defaultValues: {
      betType: 0,
      category: 0,
    },
  });

  const poolDemoImage = watch("poolImage");
  const poolStart = watch("startDate");
  const poolLock = watch("lockTime");
  const poolEnd = watch("endTime");
  const poolName = watch("name");
  const poolType = watch("betType");
  const poolDetail = watch("description");
  const poolUrl = watch("eventDetailsUrl");
  const poolOptionA = watch("optionOne");
  const poolOptionB = watch("optionTwo");
  const poolMin = watch("minBetAmount");
  const poolMax = watch("maxBetAmount");
  const poolCreatorFee = watch("creatorsFee");
  const poolCategory = watch("category");

  const { isLoading } = useCustomReadContract({
    abi: abi,
    functionName: "get_all_pools",
    address: predifiContractAddress,
    args: [],
  });

  const onSubmit: SubmitHandler<creatorInputs> = (data) => {
    console.log(startDate, endDate, lockDate, data);
    setIsOpen(true);
  };

  useCreatePool({
    poolDemoImage,
    poolEnd,
    poolStart,
    poolLock,
    setEndDate,
    setLockDate,
    setImage,
    setStartDate,
  });

  const { account } = useAccount();
  if (account) {
    prediFiContract.connect(account);
  }

  const data = {
    poolName,
    poolType,
    poolDetail,
    image: poolDemoImage,
    poolUrl,
    startDate: startDate ?? 0,
    lockDate: lockDate ?? 0,
    endDate: endDate ?? 0,
    poolOptionA,
    poolOptionB,
    account,
    poolMin,
    poolMax,
    poolCreatorFee,
    poolCategory,
  };

  async function createPoolFn() {
    await sendFn(data);
  }

  if (isLoading) return <Loading message="Fetching pool data" />;

  const betTypeOptions = [{ value: 0, label: " " }];

  return (
    <>
      {isOpen && (
        <CreatePoolModal
          sendFn={createPoolFn}
          modalHandle={() => setIsOpen(false)}
          data={data}
        />
      )}
      <section className="max-w-7xl mx-auto px-4 py-6 text-[#CCCCCC]">
        <h2 className="text-2xl font-bold mb-6">Create a pool</h2>
        <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col gap-8">
          {/* Pool Basic Info - Reverted with static status */}
          <div className="md:grid sm:grid-cols-3 gap-3 flex flex-col-reverse items-end">
            <div className="flex gap-1 flex-col text-base place-self-end w-full order-1 sm:order-none">
              <Controller
                name="name"
                control={control}
                rules={{ required: "Name is required" }}
                render={({ field, fieldState }) => (
                  <FormInput
                    label="Name"
                    // placeholder="Enter name"
                    error={fieldState.error}
                    {...field}
                  />
                )}
              />
            </div>
            <div className="flex gap-1 flex-col  w-full">
              <Controller
                name="betType"
                control={control}
                rules={{ required: "Bet type is required" }}
                render={({ field, fieldState }) => (
                  <FormSelect
                    label="Bet type"
                    options={betTypeOptions}
                    error={fieldState.error}
                    {...field}
                  />
                )}
              />
            </div>

            <div className="flex justify-center w-full xl:justify-end order-2 sm:order-none">
              <Controller
                name="poolImage"
                control={control}
                rules={{ required: "Pool image is required" }}
                render={({ field, fieldState }) => (
                  <ImageUpload
                    imageUrl={image}
                    error={fieldState.error}
                    onChange={(e) => {
                      field.onChange(e);
                    }}
                  />
                )}
              />
            </div>
          </div>

          {/* Description */}
          <Controller
            name="description"
            control={control}
            rules={{ required: "Description is required" }}
            render={({ field, fieldState }) => (
              <FormTextarea
                label="Description"
                // placeholder="Pool description"
                error={fieldState.error}
                {...field}
              />
            )}
          />

          {/* Event URL */}
          <Controller
            name="eventDetailsUrl"
            control={control}
            rules={{ required: "Event details URL is required" }}
            render={({ field, fieldState }) => (
              <FormInput
                label="Event details URL"
                // placeholder="Pool URL"
                error={fieldState.error}
                {...field}
              />
            )}
          />

          {/* Date Inputs */}
          <div className="sm:grid sm:grid-cols-3 gap-3 flex flex-col">
            <DateInput name="startDate" label="Start time" control={control} />
            <DateInput name="lockTime" label="Lock time" control={control} />
            <DateInput name="endTime" label="End time" control={control} />
          </div>

          {/* Pool Options */}
          <PoolOptions control={control} />

          {/* Bet Amount Settings */}
          <BetAmountSettings control={control} />

          {/* Submit Button */}
          <div className="flex justify-end mt-4">
            <button
              type="submit"
              className="border-[#373737] hover:bg-[#444444] text-white rounded-full border px-6 py-2 transition-colors"
            >
              Preview pool
            </button>
          </div>
        </form>
      </section>
    </>
  );
}

export default CreatePoolForm;
