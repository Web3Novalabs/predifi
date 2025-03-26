"use client";
import { ArgentMobileConnector } from "starknetkit/argentMobile";
import { WebWalletConnector } from "starknetkit/webwallet";
import { sepolia, mainnet } from "@starknet-react/chains";
import {
  argent,
  braavos,
  Connector,
  StarknetConfig,
  starkscan,
  useInjectedConnectors,
  cartridgeProvider,
} from "@starknet-react/core";
import { jsonRpcProvider } from "@starknet-react/core";
import { ReactNode, useCallback } from "react";
import { ControllerConnector } from "@cartridge/connector";
const StarknetProvider = ({ children }: { children: ReactNode }) => {
  const chains = [mainnet, sepolia];
  const { connectors: injected } = useInjectedConnectors({
    recommended: [argent(), braavos()],
    includeRecommended: "always",
  });

  console.log(process.env.NEXT_PUBLIC_ALCHEMY_API_KEY);
  const rpc = useCallback(() => {
    return {
      nodeUrl: process.env.NEXT_PUBLIC_ALCHEMY_API_KEY,
    };
  }, []);

  const cartridgeConnector = new ControllerConnector({
    rpc:cartridgeProvider().nodeUrl,
  });

  const provider = jsonRpcProvider({ rpc });

  const ArgentMobile = ArgentMobileConnector.init({
    options: {
      dappName: "Token bound explorer",
      url: "https://www.tbaexplorer.com/",
    },
    inAppBrowserOptions: {},
  });

  const connectors = [
    ...injected,
    new WebWalletConnector({
      url: "https://web.argent.xyz",
    }) as never as Connector,
    ArgentMobile as never as Connector,
    cartridgeConnector,
  ];

  return (
    <StarknetConfig
      chains={chains}
      provider={provider}
      connectors={connectors}
      explorer={starkscan}
      autoConnect
    >
      {children}
    </StarknetConfig>
  );
};

export default StarknetProvider;
