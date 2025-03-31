import { useEffect, useState } from "react";
import { useConnect } from "@starknet-react/core";
import { Connector } from "@starknet-react/core";
import { Button } from "./ui/button";
import { modal } from "@/type/type";
import CrossX from "@/svg/cross";
import ArgentIcon from "@/svg/argent";
import ControllerIcon from "@/svg/cartridge";
import Image from "next/image";

function Conectors({ setIsOpen }: modal) {
  const [clientConnectors, setClientConnectors] = useState<Connector[]>([]);
  const { connect, connectors } = useConnect();

  useEffect(() => {
    if (typeof window !== "undefined") {
      setClientConnectors(connectors);
    }
  }, [connectors]);

  return (
    <div className="relative">
      <div
        className="fixed h-screen w-full bg-black/40 backdrop-blur-md top-0 left-0"
        onClick={setIsOpen}
      />
      <div className="w-[500px] min-h-[320px] pb-6 pt-6 px-5 bg-black/80 fixed top-1/2 right-1/2 -translate-y-1/2 translate-x-1/2 rounded-xl shadow-lg border border-gray-700">
        <button className="absolute right-5 top-4 text-gray-600 hover:text-gray-900" onClick={setIsOpen}>
          <CrossX />
        </button>
        <h1 className="text-center text-lg font-semibold text-white uppercase tracking-wide">
          Select Wallet
        </h1>
        <div className="grid grid-cols-4 gap-3 mt-5">
          {clientConnectors.map((connector) => (
            <Button
              key={connector.id}
              onClick={() => {
                connect({ connector });
                setIsOpen();
              }}
              className="w-[100px] h-[100px] bg-black/20 rounded-lg flex flex-col items-center justify-center text-white hover:text-black hover:bg-gray-200 transition"
            >
              <div className="w-10 h-10 flex items-center justify-center mb-2">
                {connector.id.toLowerCase().includes("argent") ? (
                  <ArgentIcon />
                ) : connector.id.toLowerCase().includes("braavos") ? (
                  <Image src="/bravo.jpeg" alt="Braavos" width={55} height={55} className="rounded-full" />
                ) : connector.id.toLowerCase().includes("controller") ? (
                  <Image src="/cartridge.png" alt="controller" width={55} height={55} className="rounded-full" />
                ) : connector.id.toLowerCase().includes("argentmobile") ? (
                  <Image src="/sms.png" alt="Argent Mobile" width={55} height={55} />
                ) : (
                  <ControllerIcon />
                )}
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
