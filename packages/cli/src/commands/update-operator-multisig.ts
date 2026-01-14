import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import bs58 from "bs58";

import { parseAddressFlag } from "../utils/common";
import {
  OPERATOR_ROLE_NAMES,
  type OperatorRoleName,
  type OperatorStatusName,
  parseOperatorRoleFlag,
  parseOperatorStatusFlag,
} from "../utils/operator";
import {
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
  getManageOperatorInstruction,
  type OperatorManagementActionArgs,
  OperatorRole,
  OperatorStatus,
} from "jupusd-sdk";
import { findOperator } from "jupusd-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

type OperatorManagementActionInput =
  | {
      kind: "setStatus";
      status: OperatorStatus;
      statusName: OperatorStatusName;
    }
  | {
      kind: "setRole";
      role: OperatorRole;
      roleName: OperatorRoleName;
    };

type ParsedActionResult = {
  action: OperatorManagementActionInput;
  summary: string;
  details: Record<string, string>;
};

export default class UpdateOperatorMultisig extends BaseCommand {
  static summary = "Queue operator management via Squad's Multisig.";

  static description = `
This command queues a Squad's Multisig transaction that updates an existing operator PDA by changing its status or role.
The invoking signer must be a multisig member and the multisig authority must already be an enabled Admin operator.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    action: Flags.string({
      description: "Operator management action to perform.",
      required: true,
      options: ["set-status", "set-role"],
    }),
    "managed-operator-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the operator PDA to manage.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    status: Flags.string({
      description:
        "Desired status for the managed operator (set-status action). Accepts boolean-like values such as enabled/disabled, true/false, etc.",
      required: false,
      parse: async (input) => input.trim(),
    }),
    role: Flags.string({
      description: "Target role for the managed operator (set-role action).",
      required: false,
      options: [...OPERATOR_ROLE_NAMES],
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(UpdateOperatorMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const managedOperatorAuthority = parseAddressFlag(
      flags["managed-operator-authority"],
      "managed-operator-authority",
    );

    const parsedAction = this.parseActionFlags(flags);

    const operatorAccount = await findOperator(multisigAuthority.address);
    const managedOperatorAccount = await findOperator(managedOperatorAuthority);

    this.logger.info("Updating operator with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(
      `  Managed operator authority: ${managedOperatorAuthority}`,
    );
    this.logger.info(`  Managed operator PDA: ${managedOperatorAccount}`);
    this.logger.info(`  Action: ${parsedAction.summary}`);
    if (Object.keys(parsedAction.details).length > 0) {
      for (const [key, value] of Object.entries(parsedAction.details)) {
        this.logger.info(`    ${key}: ${value}`);
      }
    }

    const instruction = getManageOperatorInstruction({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      managedOperator: managedOperatorAccount,
      action: toOperatorManagementActionArgs(parsedAction.action),
    });

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(instruction, m),
    );

    const base64EncodedWireTransaction = getBase64EncodedWireTransaction(
      await partiallySignTransactionMessageWithSigners(transactionMessage),
    );
    const transaction = VersionedTransaction.deserialize(
      Buffer.from(base64EncodedWireTransaction.toString(), "base64"),
    );

    this.logger.info(
      `Inner transaction (unsigned, base58): ${bs58.encode(transaction.serialize())}`,
    );
  }

  private parseActionFlags(
    flags: Interfaces.InferredFlags<typeof UpdateOperatorMultisig.flags>,
  ): ParsedActionResult {
    switch (flags.action) {
      case "set-status": {
        const statusRaw = flags.status;
        if (statusRaw === undefined) {
          this.error("--status is required when action is set-status");
        }

        const parsed = parseOperatorStatusFlag(statusRaw, "status");
        return {
          action: {
            kind: "setStatus",
            status: parsed.status,
            statusName: parsed.name,
          },
          summary: `Set operator status to ${parsed.name}`,
          details: { status: parsed.name },
        };
      }

      case "set-role": {
        const roleRaw = flags.role;
        if (roleRaw === undefined) {
          this.error("--role is required when action is set-role");
        }

        const parsed = parseOperatorRoleFlag(roleRaw, "role");
        return {
          action: {
            kind: "setRole",
            role: parsed.role,
            roleName: parsed.name,
          },
          summary: `Set operator role to ${parsed.name}`,
          details: { role: parsed.name },
        };
      }

      default:
        this.error(`Unsupported action: ${flags.action}`);
    }
  }
}

function toOperatorManagementActionArgs(
  action: OperatorManagementActionInput,
): OperatorManagementActionArgs {
  switch (action.kind) {
    case "setStatus":
      return { __kind: "SetStatus", status: action.status };
    case "setRole":
      return { __kind: "SetRole", role: action.role };
    default:
      return assertNever(action);
  }
}

function assertNever(_value: never): never {
  throw new Error("Unexpected operator management action");
}
