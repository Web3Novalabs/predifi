import { Button } from "./ui/button";

const amounts = ["$1", "$5", "$10", "$50", "max"];

const StakeAmountButtons = () => (
  <div className="flex justify-between gap-[10px] w-full">
    {amounts.map((amount) => (
      <Button key={amount} className="rounded-[48px] bg-transparent h-[43px] py-[12px] px-[24px] border border-[#373737] hover:bg-[#FFFFFF66]">
        {amount}
      </Button>
    ))}
    <Button className="rounded-[48px] bg-transparent h-[43px] py-[12px] px-[24px] w-full max-w-[370px] border border-[#373737] hover:bg-[#FFFFFF66]">
      stake
    </Button>
  </div>
);

export default StakeAmountButtons;
