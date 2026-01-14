import { BenefactorStatus } from "jupusd-sdk";
import { parseBooleanFlag } from "./common";

export type BenefactorStatusName = "active" | "disabled";
export function parseBenefactorStatusFlag(
  raw: string,
  flagName: string,
): {
  name: BenefactorStatusName;
  status: BenefactorStatus;
} {
  const normalized = raw.trim().toLowerCase();

  if (normalized === "active") {
    return { name: "active", status: BenefactorStatus.Active };
  }

  if (normalized === "disabled" || normalized === "inactive") {
    return { name: "disabled", status: BenefactorStatus.Disabled };
  }

  try {
    const isActive = parseBooleanFlag(raw, flagName);
    return {
      name: isActive ? "active" : "disabled",
      status: isActive ? BenefactorStatus.Active : BenefactorStatus.Disabled,
    };
  } catch (_error) {
    throw new Error(
      `Invalid value for --${flagName}. Expected a value like active/disabled or true/false. Received: ${raw}`,
    );
  }
}

export function parseBenefactorFeeRateFlag(
  value: number | undefined,
  flagName: string,
): number {
  if (value === undefined) {
    throw new Error(`--${flagName} is required.`);
  }

  if (!Number.isInteger(value)) {
    throw new Error(`--${flagName} must be an integer.`);
  }

  if (value < 0 || value > 10000) {
    throw new Error(`--${flagName} must be between 0 and 10000 (inclusive).`);
  }

  return value;
}
