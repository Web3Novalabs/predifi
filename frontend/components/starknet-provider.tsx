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
} from "@starknet-react/core";
import { jsonRpcProvider } from "@starknet-react/core";
import { ReactNode, useCallback } from "react";
import { ControllerConnector } from "@cartridge/connector";
import { constants } from "starknet";

// IMPORTANT: Create the ControllerConnector outside the component
// This prevents recreation on each render
const cartridgeConnector = new ControllerConnector({
  chains: [
    { rpcUrl: "https://api.cartridge.gg/x/starknet/sepolia" },
    { rpcUrl: "https://api.cartridge.gg/x/starknet/mainnet" },
  ],
  defaultChainId: constants.StarknetChainId.SN_SEPOLIA,
});

const StarknetProvider = ({ children }: { children: ReactNode }) => {
  const chains = [mainnet, sepolia];
  const { connectors: injected } = useInjectedConnectors({
    recommended: [argent(), braavos()],
    includeRecommended: "always",
  });

  const rpc = useCallback(() => {
    return {
      nodeUrl: process.env.NEXT_PUBLIC_ALCHEMY_API_KEY,
    };
  }, []);

  const provider = jsonRpcProvider({ rpc });

  const ArgentMobile = ArgentMobileConnector.init({
    options: {
      dappName: "Token bound explorer",
      url: "https://www.tbaexplorer.com/",
    },
    inAppBrowserOptions: {},
  });

  const webWalletConnector = new WebWalletConnector({
    url: "https://web.argent.xyz",
  });

  const connectors = [
    ...injected,
    webWalletConnector as never as Connector,
    ArgentMobile as never as Connector,
    cartridgeConnector as never as Connector,
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