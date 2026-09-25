# PrediFi Technical Glossary

This glossary defines key terminology used across PrediFi smart contracts, backend indexers, and frontend applications. Terms are listed in alphabetical order.

---

### **BPS (Basis Points)**
A unit of measure equal to one hundredth of one percentage point ($1 \text{ bps} = 0.01\% = 0.0001$). In PrediFi smart contracts, basis points are used for precise integer calculation of treasury fees (e.g., $100 \text{ bps} = 1\%$) and referral cuts without floating-point precision loss.

### **Claim**
The transaction executed by a prediction participant to withdraw their proportional payout after a market pool resolves. Claims calculate winnings based on the user's stake in the winning outcome relative to the total winning outcome pool.

### **Ledger**
The discrete sequence unit of time and state on the Stellar blockchain, equivalent to a block in other network architectures. Soroban smart contract storage entries and time-to-live (TTL) thresholds are measured in ledgers.

### **Oracle**
An authorized data source or account responsible for submitting external real-world event data or price feed observations to resolve prediction market pools. Oracles enable automated resolution for price condition markets and binary prediction pools.

### **Outcome**
One of the discrete possible results defined when a prediction market pool is created (e.g., Option 0 vs Option 1). Participants stake tokens on their predicted outcome to receive proportional payouts if that outcome wins.

### **Pool**
The core prediction market entity holding total staked collateral across all outcomes. A pool maintains lifecycle state (Active, Resolved, Cancelled), fee parameters, start/end timestamps, and outcome accounting.

### **Price Condition**
A structured rule specified for oracle-driven prediction pools comparing asset prices against target thresholds. Price conditions define the asset pair, target value, and comparison operator (e.g., GreaterThan, LessThan) evaluated at resolution time.

### **Referral Cut**
The percentage share (measured in basis points) of protocol fees awarded to the address that referred a prediction market participant. Referral rewards are calculated during payout processing and distributed to referrers.

### **Resolution**
The contract state transition where a pool is closed to predictions and the winning outcome is established by an oracle or admin. Once resolved, payout ratios are locked and participants can submit claims.

### **Settlement**
The final financial reconciliation phase during resolution where total stakes, treasury fee deductions, and referrer payouts are calculated. Settlement locks pool balances into claims availability for winning outcome token holders.

### **Soroban**
The WebAssembly (WASM)-based smart contract platform on the Stellar blockchain. Soroban provides low-latency execution, predictable gas metering, and rust-based contract development for PrediFi contracts.

### **Stake**
The token collateral pledged by a user towards a specific outcome choice in a prediction pool. User stakes increase total outcome liquidity and dictate their relative share of winnings upon successful market resolution.

### **Stroop**
The smallest atomic unit of Stellar Lumens (XLM), where $1 \text{ XLM} = 10,000,000 \text{ stroops}$ ($10^{-7} \text{ XLM}$). Token amounts in Soroban contracts and backend transactions are represented in stroops as `i128` integers.

### **Treasury Fee**
The protocol-wide percentage fee (in basis points) deducted from total pool collateral or pool rewards upon market settlement. Collected treasury fees support protocol maintenance and ecosystem incentives.

### **TTL / Bump (Time-To-Live)**
Time-To-Live (TTL) is the ledger duration before a Soroban persistent or instance storage entry expires. The `bump` operation extends storage TTL to prevent state archival and ensure pool data remains active on-chain.

### **Validator**
A Stellar network node operator running the Stellar Consensus Protocol (SCP) to validate transactions and reach consensus. Validators confirm ledger state changes including Soroban contract deployments and invocations.
