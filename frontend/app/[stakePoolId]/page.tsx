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
import { useState, useRef } from "react";

interface FormData {
  eventLink: string;
  startDate: string;
  endDate: string;
  description: string;
}

function StakePoolId() {
  const { stakePoolId } = useParams<{ stakePoolId: string }>();
  const {
    register,
    formState: { errors },
  } = useForm<FormData>();

  const [formData, setFormData] = useState<FormData>({
    eventLink: "",
    startDate: "",
    endDate: "",
    description: "",
  });

  const startDateInputRef = useRef<HTMLInputElement>(null!);
  const endDateInputRef = useRef<HTMLInputElement>(null!);

  // Function to handle input changes
  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setFormData({ ...formData, [name]: value });
  };

  // Function to trigger the hidden date picker
  const openDatePicker = (ref: React.RefObject<HTMLInputElement>) => {
    ref.current?.showPicker();
  };

  return (
    <section className="md:px-10 px-5 font-['Work_Sans']">
      <div className="w-full border-b-[1px] border-[#373737] flex justify-between">
        <div className="flex gap-[13px]">
          <TabNavigation />
        </div>
        <div>
          <button className="flex items-center gap-[10px] text-[16px] font-[400] hover:bg-[#373737] text-[#CCCCCC] rounded-[5px] transition-all duration-200 px-5 py-2">
            <span className="flex flex-none">Create <span className="hidden sm:flex px-1">New</span>  Pool</span>
            <PlusIcon />
          </button>
        </div>
      </div>
      <div className="w-full flex flex-col lg:flex-row gap-3 lg:gap-[100px] mt-20">
        <div className="lg:w-[433px] relative w-full flex sm:mx-auto flex-none lg:h-[404px] h-[200px]">
          <Image
            className="w-full flex flex-none h-[100%] rounded-[4px]"
            src={img}
            alt="Pool Image"
            layout="fill"
            objectFit="cover"
          />
        </div>
        <div className="w-full gap-[40px] flex flex-col">
          <h2 className="text-2xl text-[#CCCCCC] text-[24px] font-[600]">
            PredFi to win the hackathon Pool ID: {stakePoolId}
          </h2>
          <textarea
            id="description"
            name="description"
            placeholder="You can now put your prediction into something and get outcomes"
            className="border-[#373737] placeholder:text-[#CCCCCC] bg-transparent outline-none text-[#CCCCCC] border rounded-[8px] p-3 h-[118px] max-h-[118px]"
            value={formData.description}
            onChange={handleChange}
          />
          <Input
            type="text"
            label="Event Link"
            placeholder="Link"
            id="eventLink"
            {...register("eventLink", { required: "Event Link is required" })}
            error={errors.eventLink?.message}
          />
          <div className="flex gap-[20px] relative flex-col sm:flex-row">
            <div className="flex flex-col gap-[15px] w-full">
              <h3 className="text-[#8F8F8F] text-[16px] font-[600]">Total Amount in Pool</h3>
              <span className="text-[#CCCCCC] font-[400] text-[16px]">$10,000</span>
            </div>
            <div className="flex flex-col w-full">
              <label htmlFor="startDate" className="text-[#9596A0] text-[0.875rem] mb-1">
                Start Date <span className="text-[#C04639]">*</span>
              </label>
              <div className="bg-transparent border-[1px] border-[#252625] text-white px-4 py-[0.75rem] rounded-[0.5rem]"
                onClick={() => openDatePicker(startDateInputRef)}>
                {formData.startDate || "Select date"}
              </div>
              <input
                type="date"
                name="startDate"
                ref={startDateInputRef}
                className="hidden"
                onChange={handleChange}
              />
            </div>
            <div className="flex flex-col w-full">
              <label htmlFor="endDate" className="text-[#9596A0] text-[0.875rem] mb-1">
                End Date <span className="text-[#C04639]">*</span>
              </label>
              <div className="bg-transparent border-[1px] border-[#252625] text-white px-4 py-[0.75rem] rounded-[0.5rem]"
                onClick={() => openDatePicker(endDateInputRef)}>
                {formData.endDate || "Select date"}
              </div>
              <input
                type="date"
                name="endDate"
                ref={endDateInputRef}
                className="hidden"
                onChange={handleChange}
              />
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <Button className="rounded-full bg-[#FFFFFF66] text-[16px] text-[#CCCCCC] h-[43px] border border-[#373737]">
              Yes: PredFi to win
            </Button>
            <Button className="rounded-full border border-[#373737] bg-transparent text-[16px] text-[#CCCCCC] h-[43px]">
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
          <StakeAmountButtons />
        </div>
      </div>
    </section>
  );
}

export default StakePoolId;
