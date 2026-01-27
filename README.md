# PrediFi - Decentralized Outcome Prediction Protocol (Stellar/Soroban)

PrediFi is a decentralized prediction protocol built on the **Stellar network** using **Soroban smart contracts**. In a trustless, transparent, and secure environment, it allows users to predict future outcomes across various fields, including sports, finance, and global events. By utilizing Stellar's fast and low-cost technology, PrediFi ensures that all predictions and their results are verifiable on-chain and immutable.

Telegram Community: [here](https://t.me/predifi_onchain_build/1)

## Project Overview

PrediFi is designed to empower individuals, influencers, and communities to enter the dynamic world of prediction markets. Leveraging the power of blockchain, PrediFi allows anyone to establish custom prediction markets focused on any event imaginable.

## Project Structure

The repository is organized into two main workspaces:

- `contract/`: Contains the Stellar/Soroban smart contracts (Rust).
- `frontend/`: Contains the Next.js web application (TypeScript).

### Smart Contracts (`contract/`)

The smart contract logic is written in **Rust** for the **Soroban** platform.

- `contracts/`: Directory containing individual contract crates.
  - `hello-world/`: Initial template contract (to be replaced/expanded with PrediFi logic).

### Frontend (`frontend/`)

The user interface is built with **Next.js**, **Tailwind CSS**, and **TypeScript**.

## Development

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install)
- **Soroban CLI**: [Install Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup#install-the-soroban-cli)
- **Node.js**: [Install Node.js](https://nodejs.org/)
- **pnpm**: [Install pnpm](https://pnpm.io/installation)

### Installation & Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/Web3Novalabs/predifi.git
   cd predifi
   ```

2. **Smart Contracts:**

   Navigate to the contract directory:

   ```bash
   cd contract
   ```

   Build the contracts:

   ```bash
   soroban contract build
   ```

   Run tests:

   ```bash
   cargo test
   ```

3. **Frontend:**

   Navigate to the frontend directory:

   ```bash
   cd ../frontend
   ```

   Install dependencies:

   ```bash
   pnpm install
   ```

   Run the development server:

   ```bash
   pnpm dev
   ```

   Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

## Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/your-feature-name`).
3. Commit your changes with meaningful messages.
4. Test your changes thoroughly.
5. Submit a Pull Request.

## License

[MIT](LICENSE)
