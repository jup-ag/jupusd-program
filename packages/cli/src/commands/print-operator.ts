import { Flags } from "@oclif/core";
import BaseCommand from "../base-command";
import { fetchOperator, OperatorStatus } from "jup-stable-sdk";
import { parseAddressFlag } from "../utils/common";
import { findOperator } from "jup-stable-sdk";
import { OPERATOR_ROLE_NAMES } from "../utils/operator";

type OperatorAccount = Awaited<ReturnType<typeof fetchOperator>>;

export default class PrintOperator extends BaseCommand {
  static summary = "Fetch and display a Jupiter Stable Operator account.";

  static description = `
This command derives the operator PDA for the provided authority and prints its contents.
`;

  static flags = {
    ...BaseCommand.flags,
    authority: Flags.string({
      description: "Base58 address of the operator authority.",
      required: true,
      parse: async (input) => input.trim(),
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(PrintOperator);

    this.configureRpcClients();

    const authority = parseAddressFlag(flags.authority, "authority");

    const operatorAddress = await findOperator(authority);

    this.logger.info(`Fetching operator for authority ${authority}...`);
    this.logger.info(`  Operator PDA: ${operatorAddress}`);

    let operatorAccount: OperatorAccount;
    try {
      operatorAccount = await fetchOperator(this.rpc, operatorAddress);
    } catch (error) {
      this.logger.error(
        `Failed to fetch operator account: ${(error as Error).message ?? "unknown error"}`,
      );
      process.exit(1);
    }

    const data = operatorAccount.data;
    const roleDescription = formatRole(data.role);
    const statusDescription = formatStatus(data.status);

    this.logger.info("Operator account data:");
    this.logger.info(`  Authority: ${data.operatorAuthority}`);
    this.logger.info(`  Role: ${roleDescription}`);
    this.logger.info(`  Status: ${statusDescription}`);
  }
}

function formatRole(value: bigint): string {
  const knownMask = (1n << BigInt(OPERATOR_ROLE_NAMES.length)) - 1n;
  const activeRoles: string[] = [];

  for (let index = 0; index < OPERATOR_ROLE_NAMES.length; index += 1) {
    const bit = 1n << BigInt(index);
    if ((value & bit) !== 0n) {
      activeRoles.push(`${OPERATOR_ROLE_NAMES[index]} (bit ${index})`);
    }
  }

  const unknownMask = value & ~knownMask;
  if (activeRoles.length === 0 && unknownMask === 0n) {
    return `none (raw ${value.toString()})`;
  }

  const parts: string[] = [];
  if (activeRoles.length > 0) {
    parts.push(activeRoles.join(", "));
  }
  if (unknownMask !== 0n) {
    parts.push(`unknown mask 0x${unknownMask.toString(16)}`);
  }

  return `${parts.join("; ")} (raw ${value.toString()})`;
}

function formatStatus(status: OperatorStatus): string {
  switch (status) {
    case OperatorStatus.Enabled:
      return "enabled";
    case OperatorStatus.Disabled:
      return "disabled";
    default:
      return "unknown";
  }
}
