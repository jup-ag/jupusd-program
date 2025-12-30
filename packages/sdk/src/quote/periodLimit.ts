import type { Benefactor, Config, PeriodLimit, Vault } from "../generated";

export type PeriodLimitCheckOperation = "mint" | "redeem";
export type PeriodLimitCheckInput = {
  amount: bigint | number;
  operation: PeriodLimitCheckOperation;
  benefactor: Benefactor;
  config: Config;
  vault: Vault;
};

export type PeriodLimitViolationAccount = "config" | "vault" | "benefactor";
export type PeriodLimitViolation = {
  account: PeriodLimitViolationAccount;
  remainingAmount: bigint;
};

export function findPeriodLimitViolation(
  input: PeriodLimitCheckInput,
): PeriodLimitViolation | null {
  const amount = toBigInt(input.amount, "amount");
  if (amount <= 0n) {
    throw new Error("amount must be greater than zero");
  }

  const operation = input.operation;
  if (operation !== "mint" && operation !== "redeem") {
    throw new Error('operation must be either "mint" or "redeem"');
  }

  const now = BigInt(Math.floor(Date.now() / 1000));

  const configRemaining = findLimitExceedance(
    input.config.periodLimits,
    amount,
    now,
    operation,
  );
  if (configRemaining !== null) {
    return { account: "config", remainingAmount: configRemaining };
  }

  const vaultRemaining = findLimitExceedance(
    input.vault.periodLimits,
    amount,
    now,
    operation,
  );
  if (vaultRemaining !== null) {
    return { account: "vault", remainingAmount: vaultRemaining };
  }

  const benefactorRemaining = findLimitExceedance(
    input.benefactor.periodLimits,
    amount,
    now,
    operation,
  );
  if (benefactorRemaining !== null) {
    return { account: "benefactor", remainingAmount: benefactorRemaining };
  }

  return null;
}

function findLimitExceedance(
  periodLimits: ReadonlyArray<PeriodLimit>,
  amount: bigint,
  now: bigint,
  operation: PeriodLimitCheckOperation,
): bigint | null {
  for (const limit of periodLimits) {
    const remaining = getRemainingCapacity(limit, now, operation);
    if (remaining !== null && amount > remaining) {
      return remaining > 0n ? remaining : 0n;
    }
  }

  return null;
}

function getRemainingCapacity(
  limit: PeriodLimit,
  now: bigint,
  operation: PeriodLimitCheckOperation,
): bigint | null {
  const durationSeconds = toBigInt(
    limit.durationSeconds,
    "limit.durationSeconds",
  );
  if (durationSeconds === 0n) {
    return null;
  }

  const maxAmount =
    operation === "mint"
      ? toBigInt(limit.maxMintAmount, "limit.maxMintAmount")
      : toBigInt(limit.maxRedeemAmount, "limit.maxRedeemAmount");
  if (maxAmount === 0n) {
    return 0n;
  }

  const windowStart = toBigInt(limit.windowStart, "limit.windowStart");
  const windowElapsed = now - windowStart;

  const usedAmount =
    windowElapsed >= durationSeconds
      ? 0n
      : operation === "mint"
        ? toBigInt(limit.mintedAmount, "limit.mintedAmount")
        : toBigInt(limit.redeemedAmount, "limit.redeemedAmount");

  return maxAmount - usedAmount;
}

function toBigInt(value: bigint | number, field: string): bigint {
  if (typeof value === "bigint") {
    return value;
  }

  if (!Number.isFinite(value) || !Number.isInteger(value)) {
    throw new Error(`${field} must be a finite integer`);
  }

  return BigInt(value);
}
