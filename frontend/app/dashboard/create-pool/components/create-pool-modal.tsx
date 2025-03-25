import { Button } from "@/components/ui/button";
import BackArrow from "@/svg/back-arrow"
import { poolModal } from "@/type/type";
import Image from "next/image";

function CreatePoolModal({modalHandle,sendFn,data}:poolModal){
  console.log(data)
    return (
      <div className="relative">
        <div
          className="fixed h-screen w-full bg-[#868686]/20 backdrop-blur-md top-0 left-0"
          onClick={modalHandle}
        />
        <div className="w-[90%] max-h-[500px] grid mmin-h-[467px]  z-50 h-fit  top-1/2 right-1/2 fixed -translate-y-[50%] translate-x-[50%] rounded-lg border  py-6 px-4 overflow-scroll max-w-5xl">
          <div
            className=" border-b border-[#373737] flex items-center p-2 gap-2 mb-5"
            onClick={modalHandle}
          >
            <BackArrow />
            <span>Go back</span>
          </div>
          <div className="flex flex-col md:flex-row justify-between gap-7 md:border md:border-[#373737] md:p-2 rounded-sm">
            <div className="w-full h-[150px] md:w-[420px] md:h-[344px] relative border">
              <Image src="" alt="pool-image" fill />
            </div>
            <div>
              <h2 className="text-xl sm:text-2xl font-work font-semibold"></h2>
              <div className="flex flex-col md:flex-row">
                <div className="grid gap-5 md:gap-10 md:w-1/2">
                  <div>
                    <h3>{data.poolName}</h3>
                    <h3>{data.poolDetail}</h3>
                  </div>
                  <div>
                    <h3>
                      Start Time: <span>01 - 03 - 2025</span>
                    </h3>
                    <h3>
                      Lock Time: <span>01 - 03 - 2025</span>
                    </h3>
                    <h3>
                      End Time: <span>01 - 03 - 2025</span>
                    </h3>
                  </div>
                  <div className="grid self-start gap-5 md:hidden">
                    <div>
                      <h3>Event URL</h3>
                      <h3>{data.poolUrl}</h3>
                    </div>
                    <div>
                      <h3>Total Amount in Pool</h3>
                      <h3>$10,000</h3>
                    </div>
                  </div>
                  <div className="grid gap-2">
                    <div className="grid gap-2 grid-cols-2">
                      <Button className="rounded-full">
                        Yes: PredFi to win
                      </Button>
                      <Button className="rounded-full">$10</Button>
                    </div>
                    <Button onClick={sendFn} className="w-full rounded-full">
                      Commit
                    </Button>
                  </div>
                </div>
                <div className="w-1/2 md:grid place-content-end self-start gap-16 hidden ">
                  <div>
                    <h3>Event URL</h3>
                    <h3>https://earn.superteam.fun/all/</h3>
                  </div>
                  <div>
                    <h3>Total Amount in Pool</h3>
                    <h3>$10,000</h3>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
}
export default CreatePoolModal