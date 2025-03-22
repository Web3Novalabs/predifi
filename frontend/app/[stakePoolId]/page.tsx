"use client";
import Image from "next/image";
import img from "@/public/Image.svg";
import { Button } from "@/components/ui/button";
import { useParams } from "next/navigation";
import PlusIcon from "@/svg/plus-icon";
import Input from "@/components/ui/input";
import { useForm } from "react-hook-form";
import StakeAmountButtons from "@/components/StakeAmountButtons";
import TabNavigation from "@/components/tabNavigation";

function StakePoolId() {
  const { stakePoolId } = useParams();

  const {
    register,
    formState: { errors },
  } = useForm();

 

  return (
    <section className="md:px-10 px-5 font-['Work_Sans']">
      <div className="w-full border-b-[1px] border-[#373737] flex justify-between flex-col sm:flex-row ">
        <div className="flex gap-[13px]">
          <TabNavigation />
        </div>
        <div>
          <button className="flex items-center gap-[10px] text-[16px] font-[400] hover:bg-[#373737] text-[#CCCCCC] rounded-[5px] transition-all duration-200 px-5 py-2">
            <span>Create New Pool</span>
            <PlusIcon className="w-[24px] h-[24px]" />
          </button>
        </div>
      </div>
      <div className=" w-full flex sm:flex-col lg:flex-row gap-3 mt-20">
        <div className="md:w-[433px] w-full hidden lg:flex sm:mx-auto flex-none h-[404px] ">
          <Image
            src={img}
            className="w-full flex flex-none h-full rounded-[4px] object-fill "
            alt="pool image"
            width={100}
            height={100}
          />
        </div>
        <div className="w-full gap-[40px]  flex flex-col">
          <div>
            <h2 className="font-semibold font-work text-2xl">
              PredFi to win the hackathon Pool ID: {stakePoolId}
            </h2>
          </div>
          <div className="flex flex-col gap-3">
            <label
              htmlFor="description"
              className="text-[#8F8F8F]  text-[16px] font-[600]">
              Description
            </label>
            <textarea
              id="description"
              placeholder=" You can now put your prediction into something and get outcomes"
              className="border-[#373737] placeholder:text-[#CCCCCC] bg-transparent outline-none text-[#CCCCCC] border rounded-[8px]  p-3 h-[118px] max-h-[118px]"
            />
          </div>
          <Input
            type="text"
            label="Event Link"
            placeholder="Link"
            id="Event Link"
            {...register("eventLink", { required: "Event Link is required" })}
            error={errors.eventLink?.message?.toString()}
          />
          <div className="grid gap-3 grid-cols-3">
            <div className="cols-span-1 gap-3">
              <h2 className="text-[#8F8F8F] font-semibold">
                Total Amount in Pool
              </h2>
              <h3 className="text-[#CCCCCC]">$10,000</h3>
            </div>
            <Input
              type="date"
              label="Start Time"
              placeholder="01 - 03 - 2025"
              id="start"
              {...register("startTime", { required: "Start Time is required" })}
              error={errors.startTime?.message?.toString()}
            />
            <Input
              type="date"
              label="Stop Time"
              placeholder="01 - 03 - 2025"
              id="end"
              {...register("endTime", { required: "End Time is required" })}
              error={errors.endTime?.message?.toString()}
            />
          </div>

          <div className="grid gap-5">
            <div className="grid grid-cols-2 gap-3">
              <Button className="rounded-full bg-[#FFFFFF66] text-[16px] font-[400] text-[#CCCCCC] h-[43px] border border-[#373737]">
                Yes: PredFi to win
              </Button>
              <Button className="rounded-full border border-[#373737] bg-transparent text-[16px] font-[400] text-[#CCCCCC] h-[43px]">
                No: PredFi not to win
              </Button>
            </div>
            <div className="relative">
              <span className="absolute left-4 top-3">$</span>
              <input
                type="number"
                className="bg-transparent outline-none border-[#373737] rounded-full border w-full px-7 py-3"
              />
            </div>
            <div className="flex justify-between  gap-[10px]">
            <StakeAmountButtons />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
export default StakePoolId;
