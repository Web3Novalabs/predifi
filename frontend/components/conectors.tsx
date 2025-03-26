import { useConnect } from "@starknet-react/core";
import { Button } from "./ui/button";
import { modal } from "@/type/type";
import CrossX from "@/svg/cross";
import ArgentIcon from "@/svg/argent";
import BraavosIcon from "@/public/bravo.jpeg";
import WebIcon from "@/public/sms.png";
import ControllerIcon from "@/svg/cartridge"; // Example additional icon

function Conectors({ setIsOpen }: modal) {
  const { connect, connectors } = useConnect();

  return (
    <div className="relative">
      <div
        className="fixed h-screen w-full bg-black/40 backdrop-blur-md top-0 left-0"
        onClick={setIsOpen}
      />
      <div className="w-[500px] min-h-[320px] pb-6 pt-6 px-5 bg-white fixed top-1/2 right-1/2 -translate-y-1/2 translate-x-1/2 rounded-xl shadow-lg border border-gray-300">
        <button className="absolute right-5 top-4 text-gray-600 hover:text-gray-900" onClick={setIsOpen}>
          <CrossX />
        </button>
        <h1 className="text-center text-lg font-semibold text-black uppercase tracking-wide">
          Select Wallet
        </h1>
        <div className="grid grid-cols-4 gap-3 mt-5">
          {connectors.map((connector) => (
            <Button
              key={connector.id}
              onClick={() => {
                connect({ connector });
                setIsOpen();
              }}
              className="w-[85px] h-[85px] bg-gray-100 rounded-lg flex flex-col items-center justify-center text-black hover:bg-gray-200 transition"
            >
              <div className="w-10 h-10 flex items-center justify-center mb-1">
                {connector.id === "argent" ? (
                  <ArgentIcon />
                ) : connector.id === "braavos" ? (
                  <img src={BraavosIcon.src} alt="Braavos" className="w-8 h-8 rounded-full" />
                ) : connector.id === "controller" ? (
                  <ControllerIcon />
                ) : connector.id === "ArgentMobile" ? (
                  <img src={WebIcon.src} alt="webicon" />
                ):<ControllerIcon />}
              </div>
              <span className="text-xs font-medium text-center">{connector.id}</span>
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
}

export default Conectors;
