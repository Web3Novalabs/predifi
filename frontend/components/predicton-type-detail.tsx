import img1 from "@/public/why-choose-1.png";
import img2 from "@/public/why-choose-2.png";
import img3 from "@/public/why-choose-3.png";
import img4 from "@/public/why-choose-4.png";
import Image from "next/image";

function PredictionType() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2  lg:grid-cols-2 justify-center gap-5 px-5 md:px-10 xl:px-[100px]">
      <div className="border rounded-[8px]  flex flex-col lg:flex-row justify-between items-center gap-3 sm:gap-4 w-full font-work lg:pr-3">
        <Image className="w-full h-full rounded-[8px] lg:self-start" src={img1} alt="" />
        <div className="grid gap-4 p-5 lg:p-0">
          <h2 className="capitalize font-semibold xl:text-lg">
            Decentralized and Transparent
          </h2>
          <p className="font-normal  text-xs">
            The Win Bet is a straightforward prediction pool where participants
            choose between two clear outcomes.
          </p>
          <button className="w-fit border border-[#373737] px-3 py-[1px] sm:py-1 text-center rounded-full capitalize">
            Learn more
          </button>
        </div>
      </div>
      <div className="border  rounded-[8px] flex flex-col lg:flex-row justify-between items-center gap-3 sm:gap-4 w-full font-work lg:pr-3">
        <Image className="w-full h-full self-start rounded-[8px]" src={img2} alt="" />
        <div className="grid gap-4 p-5 lg:p-0">
          <h2 className="capitalize font-semibold xl:text-lg">
            No Coding Required
          </h2>
          <p className="font-normal text-xs">
            The Win Bet is a straightforward prediction pool where participants
            choose between two clear outcomes.
          </p>
          <button className="w-fit border border-[#373737] px-3 py-[1px] sm:py-1 text-center rounded-full capitalize">
            Learn more
          </button>
        </div>
      </div>
      <div className="border  rounded-[8px] flex flex-col lg:flex-row justify-between items-center gap-3 sm:gap-4 w-full font-work lg:pr-3">
        <Image className="w-full h-full self-start rounded-[8px]" src={img3} alt="" />
        <div className="grid gap-4 p-5 lg:p-0">
          <h2 className="capitalize font-semibold xl:text-lg">
            Profit While Engaging
          </h2>
          <p className="font-normal text-xs">
            The Win Bet is a straightforward prediction pool where participants
            choose between two clear outcomes.
          </p>
          <button className="w-fit border border-[#373737] px-3 py-[1px] sm:py-1 text-center rounded-full capitalize">
            Learn more
          </button>
        </div>
      </div>
      <div className="border  rounded-[8px] flex flex-col lg:flex-row justify-between items-center gap-3 sm:gap-4 w-full font-work lg:pr-3">
        <Image className="w-full h-full self-start rounded-[8px]" src={img4} alt="" />
        <div className="grid gap-4 p-5 lg:p-0">
          <h2 className="capitalize font-semibold xl:text-lg">
            Decentralized and Transparent
          </h2>
          <p className="font-normal text-xs">
            The Win Bet is a straightforward prediction pool where participants
            choose between two clear outcomes.
          </p>
          <button className="w-fit border border-[#373737] px-3 py-[1px] sm:py-1 text-center rounded-full capitalize">
            Learn more
          </button>
        </div>
      </div>
    </div>
  );
}

export default PredictionType;