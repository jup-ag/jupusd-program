import BaseCommand from "../base-command";
import {
  getOperatorDecoder,
  JUP_STABLE_PROGRAM_ADDRESS,
  OPERATOR_DISCRIMINATOR,
  OperatorStatus,
} from "jup-stable-sdk";
import { OPERATOR_ROLE_NAMES } from "../utils/operator";
import { Base64EncodedBytes } from "@solana/kit";

export default class PrintOperator extends BaseCommand {
  static summary = "Fetch and display all Operators account.";

  static description = `
This command derives the operator PDA for the provided authority and prints its contents.
`;

  static flags = {
    ...BaseCommand.flags,
  };

  async run(): Promise<void> {
    this.configureRpcClients();

    const discriminator = Buffer.from(OPERATOR_DISCRIMINATOR).toString(
      "base64",
    ) as Base64EncodedBytes;
    const accounts = await this.rpc
      .getProgramAccounts(JUP_STABLE_PROGRAM_ADDRESS, {
        filters: [
          {
            memcmp: {
              offset: 0n,
              encoding: "base64",
              bytes: discriminator,
            },
          },
        ],
        encoding: "base64",
      })
      .send();

    const decoder = getOperatorDecoder();

    this.logger.info(`Found ${accounts.length} operators`);

    for (const account of accounts) {
      const pubkey = account.pubkey;
      const data = decoder.decode(
        Buffer.from(account.account.data[0], "base64"),
      );
      const roleDescription = formatRole(data.role);
      const statusDescription = formatStatus(data.status);

      this.logger.info(`Operator account (${pubkey.toString()}):`);
      this.logger.info(`  Authority: ${data.operatorAuthority}`);
      this.logger.info(`  Role: ${roleDescription}`);
      this.logger.info(`  Status: ${statusDescription}`);
      this.logger.info(``);
    }
    // const data = operatorAccount.data;
    // const roleDescription = formatRole(data.role);
    // const statusDescription = formatStatus(data.status);

    // this.logger.info("Operator account data:");
    // this.logger.info(`  Authority: ${data.operatorAuthority}`);
    // this.logger.info(`  Role: ${roleDescription}`);
    // this.logger.info(`  Status: ${statusDescription}`);
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
