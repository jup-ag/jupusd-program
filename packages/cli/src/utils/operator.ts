import { OperatorRole, OperatorStatus } from "jupusd-sdk";
import { parseBooleanFlag } from "./common";

export const OPERATOR_SEED = "operator";

export const OPERATOR_ROLE_NAMES = [
  "admin",
  "period-manager",
  "global-disabler",
  "vault-manager",
  "vault-disabler",
  "benefactor-manager",
  "benefactor-disabler",
  "peg-manager",
  "collateral-manager",
] as const;

export type OperatorRoleName = (typeof OPERATOR_ROLE_NAMES)[number];

export const ROLE_NAME_TO_ROLE: Record<OperatorRoleName, OperatorRole> = {
  admin: OperatorRole.Admin,
  "period-manager": OperatorRole.PeriodManager,
  "global-disabler": OperatorRole.GlobalDisabler,
  "vault-manager": OperatorRole.VaultManager,
  "vault-disabler": OperatorRole.VaultDisabler,
  "benefactor-manager": OperatorRole.BenefactorManager,
  "benefactor-disabler": OperatorRole.BenefactorDisabler,
  "peg-manager": OperatorRole.PegManager,
  "collateral-manager": OperatorRole.CollateralManager,
};

export type OperatorStatusName = "enabled" | "disabled";

export function parseOperatorRoleFlag(
  value: string,
  flagName = "role",
): {
  name: OperatorRoleName;
  role: OperatorRole;
} {
  const normalized = value.trim().toLowerCase() as OperatorRoleName;
  if (!(normalized in ROLE_NAME_TO_ROLE)) {
    throw new Error(`Unsupported ${flagName} value: ${value}`);
  }

  return {
    name: normalized,
    role: ROLE_NAME_TO_ROLE[normalized],
  };
}

export function parseOperatorStatusFlag(
  raw: string,
  flagName: string,
): {
  name: OperatorStatusName;
  status: OperatorStatus;
} {
  const isEnabled = parseBooleanFlag(raw, flagName);
  return {
    name: isEnabled ? "enabled" : "disabled",
    status: isEnabled ? OperatorStatus.Enabled : OperatorStatus.Disabled,
  };
}
