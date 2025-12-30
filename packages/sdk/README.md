# Jup Stable SDK

TypeScript utilities for integrating with the JupUSD program on Solana.
The package bundles generated account and instruction helpers together
with local quoting function that mirror the program logic.

## IDL

The program IDL is available onchain: [Program IDL (Solscan)](https://solscan.io/account/JUPUSDecMzAVgztLe6eGhwUBj1Pn3j9WAXwmtHmfbRr#programIdl).

## Installation

```sh
pnpm add jup-stable-sdk @solana/kit
# or
npm install jup-stable-sdk @solana/kit
```

## Getting Started

### 1. Fetch on-chain state

The generated account helpers fetch and decode data into ergonomic TypeScript
types.

```ts
import { fetchConfig, fetchVault, fetchBenefactor } from "jup-stable-sdk";

const [config, vault, benefactor] = await Promise.all([
  fetchConfig(rpc, configAddress),
  fetchVault(rpc, vaultAddress),
  fetchBenefactor(rpc, benefactorAddress),
]);
```

### 2. Normalize oracle pricing

Quotes expect oracle prices in base units with `QUOTE_PRICE_DECIMALS` (8
decimals). The helper below matches the conversion used by the API service.

```ts
import { QUOTE_PRICE_DECIMALS } from "jup-stable-sdk";

function convertPythPriceToBigInt(price: { price: string; expo: number }) {
  const raw = BigInt(price.price);
  const scale = 10n ** BigInt(QUOTE_PRICE_DECIMALS);

  if (price.expo === 0) {
    return raw * scale;
  }

  const divisor = 10n ** BigInt(-price.expo);
  return (raw * scale) / divisor;
}
```

### 3. Produce mint and redeem quotes

```ts
import { getMintQuote, getRedeemQuote } from "jup-stable-sdk";

const amountIn = 1_000_000_000n; // 1,000 USDC with 6 decimals
const oraclePriceUsd = convertPythPriceToBigInt(latestPythPrice);

const mintQuote = getMintQuote({
  amountIn,
  config: config.data,
  benefactor: benefactor.data,
  vault: vault.data,
  oraclePriceUsd,
});

const redeemQuote = getRedeemQuote({
  amountIn,
  config: config.data,
  benefactor: benefactor.data,
  vault: vault.data,
  oraclePriceUsd,
});

console.log({
  mintOut: mintQuote.mintAmount.toString(),
  redeemOut: redeemQuote.redeemAmount.toString(),
});
```

## 4 - Build a mint / redeem transaction

The key detail is: **fetch the `Vault` account first**, then read `vault.data.oracles` to know which **oracle account(s)** to pass as remaining accounts.

```ts
import {
  AccountRole,
  address,
  appendTransactionMessageInstruction,
  createNoopSigner,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
} from "@solana/kit";
import {
  findConfig,
  fetchVault,
  findVault,
  getMintInstructionAsync,
  getRedeemInstructionAsync,
  JUP_STABLE_PROGRAM_ADDRESS,
} from "jup-stable-sdk";
import { findAssociatedTokenPda } from "@solana-program/token";

const userSigner = createNoopSigner(userAddress);
const feePayerSigner = userSigner;

// Derive config account
const configAddress = await findConfig();

// Derive vault pubkey from collateral
const vaultAddress = await findVault(USDC);

// Fetch the vault so you can also fetch the oracle account from it.
const vaultAccount = await fetchVault(rpc, vaultAddress);

// Fetch the vault so you can also fetch the oracle account from it.
const configAccount = await fetchConfig(rpc, configAddress);

// Add the oracle accounts required by this vault.
const remainingAccounts = vaultAccount.data.oracles.flatMap((oracle) => {
  if (oracle.__kind === "Empty") return [];
  const oracleAccount = oracle.fields[0].account;
  return [
    {
      role: AccountRole.READONLY,
      address: address(oracleAccount),
    },
  ];
});

// --- Mint (deposit collateral, receive Stablecoin) ---
const mintIx = await getMintInstructionAsync({
  user: userSigner,
  userCollateralTokenAccount,
  userLpTokenAccount: userLpTokenAccount,
  config: configAddress,
  authority: configAccount.data.authority,
  lpMint: configAccount.data.mint,
  vault: vaultAddress,
  vaultMint: vaultAccount.data.mint,
  custodian: vaultAccount.data.custodian,
  custodianTokenAccount: await findAssociatedTokenPda({
    mint: vaultAccount.data.mint,
    owner: vaultAccount.data.custodian,
    tokenProgram: vaultAccount.data.token_program,
  }),
  benefactor: benefactorAddress,
  lpTokenProgram: configAccount.data.tokenProgram,
  vaultTokenProgram: vaultAccount.data.tokenProgram,
  program: JUP_STABLE_PROGRAM_ADDRESS,
  amount: amountIn,
  minAmountOut,
});
mintIx.accounts.push(...remainingAccounts);

// --- Redeem (burn Stablecoin, withdraw collateral) ---
const redeemIx = await getRedeemInstructionAsync({
  user: userSigner,
  userLpTokenAccount,
  userCollateralTokenAccount,
  config: configAddress,
  authority: configAccount.data.authority,
  lpMint: configAccount.data.mint,
  vault: vaultAddress,
  vaultTokenAccount,
  vaultMint: vaultAccount.data.mint,
  benefactor: benefactorAddress,
  lpTokenProgram: configAccount.data.tokenProgram,
  vaultTokenProgram: vaultAccount.data.tokenProgram,
  program: JUP_STABLE_PROGRAM_ADDRESS,
  amount: amountIn,
  minAmountOut,
});
redeemIx.accounts.push(...remainingAccounts);

// Build a v0 transaction message (same pattern as the CLI).
const { value: blockhash } = await rpc.getLatestBlockhash().send();
const txMessage = pipe(
  createTransactionMessage({ version: 0 }),
  (m) => setTransactionMessageFeePayerSigner(feePayerSigner, m),
  (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
  (m) => appendTransactionMessageInstruction(mintIx /* or redeemIx */, m),
);

const serialized = getBase64EncodedWireTransaction(
  await partiallySignTransactionMessageWithSigners(txMessage),
).toString();
```

## PDA helpers

Deterministic PDAs are exposed through the derivation utilities.

```ts
import {
  findBenefactor,
  findConfig,
  findOperator,
  findVault,
  findVaultTokenAccount,
} from "jup-stable-sdk";

const [configAddress, vaultAddress, benefactorAddress] = await Promise.all([
  findConfig(),
  findVault(collateralMint),
  findBenefactor(authorityAddress),
]);

const operatorAddress = await findOperator(authorityAddress);
const vaultTokenAccount = await findVaultTokenAccount(collateralMint);
```

- `findConfig()` derives the global configuration PDA.
- `findVault(mint)` derives the vault PDA for a given collateral.
- `findVaultTokenAccount(mint)` derives the vault-owned token account that.
- `findBenefactor(authority)` and `findOperator(authority)` derives PDAs tied to
  authority addresses.
