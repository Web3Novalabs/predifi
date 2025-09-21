# PrediFi - Decentralized outcome prediction protocol (on-chain prediction platform )

Telegram Community: [here](https://t.me/predifi_onchain_build/1)
## Project Overview:
PrediFi is a decentralized prediction protocol built on StarkNet. In a trustless, transparent, and secure environment, it allows users to predict future outcomes across various fields, including sports, finance, and global events. By utilizing starknet technology, PrediFi ensures that all predictions and their results are verifiable onchain and immutable, thus eliminating the need for intermediaries.

PrediFi is a groundbreaking decentralized platform designed to empower individuals, influencers, and communities to enter the dynamic world of prediction markets. Leveraging the transformative power of blockchain technology, PrediFi allows anyone to establish custom prediction markets focused on any event imaginable. This innovative approach provides a lively, engaging, and rewarding way to foster interaction within your community while monetizing the buzz and excitement surrounding trending topics.

In our fast-paced digital age, conversations about predictions are captivating; they span a wide range of subjects, from sports matchups and political elections to the latest pop culture phenomena. Imagine if you could transform those engaging discussions into tangible rewards! With PrediFi, you have the opportunity to create markets where individuals can wager on the outcomes of these events, turning their insights and forecasts into real-world returns.

PrediFi makes it easy to create prediction pools for a wide range of cultural and local events. You can set up pools for major sports championships and awards shows, but that's just the beginning. It's also perfect for engaging with the latest viral trends, community events, environmental happenings, and anything else that sparks buzz in your area. Whether it’s predicting the outcome of a local music festival or the next viral sensation.

## Contract Structure

The PrediFi protocol is modular and organized for clarity, security, and extensibility. Below is an overview of the main contract files and their purposes:

- `src/predifi.cairo`: Main contract logic, including pool management, staking, validation, and dispute resolution.
- `src/interfaces/IERC20.cairo`: ERC20 token interface for STRK and other tokens.
- `src/interfaces/ipredifi.cairo`: Main protocol interface, including pool management, dispute, and validator traits.
- `src/interfaces/iUtils.cairo`: Utility interface (e.g., for price feeds).
- `src/base/types.cairo`: Enums and structs for pools, statuses, categories, odds, stakes, and validators.
- `src/base/events.cairo`: All protocol events (e.g., BetPlaced, UserStaked, PoolResolved).
- `src/base/errors.cairo`: Centralized error messages and codes for all protocol operations.

## Developer Documentation & NatSpec

All public and external functions, types, and events are documented using Cairo NatSpec comments for auditability and developer clarity.

**Example:**
```cairo
/// @notice Places a bet on a pool.
/// @dev Transfers tokens from user, updates odds, and emits BetPlaced event.
/// @param pool_id The pool ID.
/// @param option The option to bet on.
/// @param amount The amount to bet.
fn vote(ref self: ContractState, pool_id: u256, option: felt252, amount: u256) { ... }
```

**Guidelines:**
- Use `@notice` for a summary of the function/type/event.
- Use `@dev` for developer/auditor notes.
- Use `@param` and `@return` for all parameters and return values.
- All new public/external functions must include NatSpec comments.


## Development:

Requirements:
- Rust
- Cairo
- Starknet foundry
- Node
- Pnpm

## Installation Guide:

Step 1:

1. Fork the repo

2. Clone the forked repo to your local machine 
  ``` bash
  git clone https://github.com/your-user-name/auto-swap
  ```

3. Setup contract:

  ```
  cd contracts
  ```
  
  // Install asdf scarb and starknet foundry:
  
  ``` bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.starkup.dev | sh
  ```
  
  // Method 2:
  
  Install asdf and install scarb, and starknet foundry: https://foundry-rs.github.io/starknet-foundry/getting-started/installation.html

4. Add development tools
  ``` bash
  asdf set --home scarb 2.9.2
  
  asdf set --home starknet-foundry 0.36.0
  
  ```
   
5. Ensure installed properly

 ``` bash
 snforge --version

 scarb --version
 ```

6. Build
``` bash
scarb build
```
7. Test
``` bash
snforge test
```
## 🐳 DevContainer Setup (Docker)

We provide a **Docker DevContainer** to simplify development and avoid local dependency issues.

### Prerequisites
- [Docker](https://docs.docker.com/get-docker/) installed and running  
- [Visual Studio Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

### Setup
1. Open the project in **VS Code**.  
2. Press **CTRL + SHIFT + P** → select **“Dev Containers: Reopen in Container”**.  
3. The container will build and install all required tools automatically.  

### Verify
Inside the container, run:

```bash
scarb build
scarb test
scarb fmt    
scarb fmt --check
```
### Note :
  - If During running `scarb build` throw error `killed` , Then increase your docker ram allocation and cores. and restart !
  
# Contributing

We welcome contributions! Please follow these steps:

## Getting Started

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/your-feature-name`)
3. Commit your changes with meaningful messages (`git commit -m 'feat: add new capability'`)
4. Test your changes thoroughly before submission

## Testing Requirements

Before submitting your PR:
1. Set up your environment variable: `export RPC_URL=https://api.cartridge.gg/x/starknet/mainnet`
2. All tests must pass locally before proceeding

## Pull Request Process

1. Ensure your branch is up to date with main (`git pull origin main`)
2. Include comprehensive test cases covering your changes
3. Update documentation to reflect your modifications
4. Provide a detailed description in your PR explaining:
   - The problem solved
   - Implementation approach
   - Any potential impacts
5. Request review from project maintainers

## Code Standards

- Follow the existing code style and conventions
- Write clean, readable, and maintainable code
- Include comments for complex logic
- Keep commits focused and atomic
- **All new public/external functions must include NatSpec comments**

## Support

Need help with your contribution? You can:
- Open an issue in the GitHub repository
- Join our Telegram channel for community assistance 
- Check existing documentation and discussions

We aim to review all contributions promptly and appreciate your efforts to improve the project!

## Security & Auditing

- All critical logic is documented with NatSpec.
- Please report vulnerabilities responsibly.

**For more details, see the inline NatSpec documentation in each contract file.**