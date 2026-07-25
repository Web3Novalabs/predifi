interface EthereumProvider {
  request(args: { method: "eth_chainId" }): Promise<string>;
  request(args: {
    method: "wallet_switchEthereumChain";
    params: [{ chainId: string }];
  }): Promise<null>;
  on(event: "chainChanged", handler: (chainId: string) => void): void;
  removeListener(
    event: "chainChanged",
    handler: (chainId: string) => void
  ): void;
}

declare global {
  interface Window {
    ethereum?: EthereumProvider;
  }
}

export {};
