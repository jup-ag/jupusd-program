import type { Benefactor, Config, Vault } from "../generated";

const BASIS_POINTS = 10_000n;
export const PEG_PRICE_DECIMALS = 4;
export const ORACLE_PRICE_DECIMALS = 8;
export const PEG_PRICE_SCALE = 10n ** BigInt(PEG_PRICE_DECIMALS);
export const ORACLE_PRICE_SCALE = 10n ** BigInt(ORACLE_PRICE_DECIMALS);

export type MintQuoteInput = {
  amountIn: bigint | number;
  config: Config;
  benefactor: Benefactor;
  vault: Vault;
  oraclePriceUsd: bigint | number;
};

export type MintQuote = {
  amountIn: bigint;
  feeAmount: bigint;
  netAmount: bigint;
  oracleAmount: bigint;
  oneToOneAmount: bigint;
  mintAmount: bigint;
};

export function ceilBigInt(n: bigint, d: bigint): bigint {
  return n / d + (n % d ? 1n : 0n);
}

export function getMintQuote(input: MintQuoteInput): MintQuote {
  const amountIn = toBigInt(input.amountIn);
  if (amountIn <= 0n) {
    throw new Error("amountIn must be greater than zero");
  }

  const pegPriceUsd = toBigInt(input.config.pegPriceUsd);
  if (pegPriceUsd <= 0n) {
    throw new Error("config.pegPriceUsd must be greater than zero");
  }

  const oraclePriceUsd = toBigInt(input.oraclePriceUsd);
  if (oraclePriceUsd <= 0n) {
    throw new Error("oraclePriceUsd must be greater than zero");
  }

  const minOraclePrice =
    toBigInt(input.vault.minOraclePriceUsd) *
    10n ** BigInt(ORACLE_PRICE_DECIMALS - PEG_PRICE_DECIMALS);
  if (oraclePriceUsd < minOraclePrice) {
    throw new Error(
      "oraclePriceUsd is outside of the vault's configured bounds",
    );
  }

  const vaultDecimals = ensureNonNegativeInteger(
    input.vault.decimals,
    "vault.decimals",
  );
  const lpMintDecimals = ensureNonNegativeInteger(
    input.config.decimals,
    "config.decimals",
  );

  const vaultScale = pow10(vaultDecimals);
  const lpScale = pow10(lpMintDecimals);

  const feeAmount = computeMintFee(amountIn, input.benefactor.mintFeeRate);
  if (feeAmount > amountIn) {
    throw new Error("Mint fee exceeds the provided amount");
  }

  const netAmount = amountIn - feeAmount;

  const oracleAmount = computeOracleAmount(
    amountIn,
    oraclePriceUsd,
    pegPriceUsd,
    lpScale,
    vaultScale,
  );
  const oneToOneAmount = computeOneToOneAmount(
    netAmount,
    pegPriceUsd,
    lpScale,
    vaultScale,
  );

  const mintAmount =
    oracleAmount < oneToOneAmount ? oracleAmount : oneToOneAmount;

  return {
    amountIn,
    feeAmount,
    netAmount,
    oracleAmount,
    oneToOneAmount,
    mintAmount,
  };
}

function computeMintFee(amount: bigint, mintFeeRate: number): bigint {
  if (!Number.isInteger(mintFeeRate) || mintFeeRate < 0) {
    throw new Error("mintFeeRate must be a non-negative integer");
  }

  const rate = BigInt(mintFeeRate);
  return ceilBigInt(amount * rate, BASIS_POINTS);
}

function computeOracleAmount(
  amount: bigint,
  oraclePriceUsd: bigint,
  pegPriceUsd: bigint,
  lpScale: bigint,
  vaultScale: bigint,
): bigint {
  if (pegPriceUsd === 0n) {
    throw new Error("pegPriceUsd cannot be zero");
  }

  return (
    (amount * oraclePriceUsd * lpScale * PEG_PRICE_SCALE) /
    (pegPriceUsd * vaultScale * ORACLE_PRICE_SCALE)
  );
}

function computeOneToOneAmount(
  netAmount: bigint,
  pegPriceUsd: bigint,
  lpScale: bigint,
  vaultScale: bigint,
): bigint {
  if (pegPriceUsd === 0n) {
    throw new Error("pegPriceUsd cannot be zero");
  }

  return (netAmount * PEG_PRICE_SCALE * lpScale) / (pegPriceUsd * vaultScale);
}

function pow10(exponent: number): bigint {
  if (!Number.isInteger(exponent) || exponent < 0) {
    throw new Error("Decimal exponents must be a non-negative integer");
  }

  return 10n ** BigInt(exponent);
}

function ensureNonNegativeInteger(value: number, field: string): number {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${field} must be a non-negative integer`);
  }

  return value;
}

function toBigInt(value: bigint | number): bigint {
  if (typeof value === "bigint") {
    return value;
  }

  if (!Number.isFinite(value) || !Number.isInteger(value)) {
    throw new Error(
      "Numeric inputs must be finite integers to avoid precision loss",
    );
  }

  return BigInt(value);
}
