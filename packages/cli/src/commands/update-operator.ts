import { Flags, Interfaces } from "@oclif/core";
import {
  appendTransactionMessageInstruction,
  assertIsSendableTransaction,
  assertIsTransactionSigner,
  assertIsTransactionWithBlockhashLifetime,
  createTransactionMessage,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from "@solana/kit";
import {
  getManageOperatorInstruction,
  type OperatorManagementActionArgs,
  OperatorRole,
  OperatorStatus,
} from "jup-stable-sdk";

import BaseCommand from "../base-command";
import {
  OPERATOR_ROLE_NAMES,
  type OperatorRoleName,
  type OperatorStatusName,
  parseOperatorRoleFlag,
  parseOperatorStatusFlag,
} from "../utils/operator";
import { findOperator } from "jup-stable-sdk";
import { parseAddressFlag } from "../utils/common";

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

export default class UpdateOperator extends BaseCommand {
  static summary = "Update an existing operator account.";

  static description = `
This command invokes the manage_operator instruction to modify the status or role of an existing operator PDA.
The invoking signer must be an enabled operator with the Admin role.
`;

  static flags = {
    ...BaseCommand.flags,
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
    const { flags } = await this.parse(UpdateOperator);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const managedOperatorAuthority = parseAddressFlag(
      flags["managed-operator-authority"],
      "managed-operator-authority",
    );

    const parsedAction = this.parseActionFlags(flags);

    const operatorAccount = await findOperator(operatorSigner.address);
    const managedOperatorAccount = await findOperator(managedOperatorAuthority);

    this.logger.info("Updating operator with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
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

    const manageOperatorInstruction = getManageOperatorInstruction({
      operatorAuthority: operatorSigner,
      operator: operatorAccount,
      managedOperator: managedOperatorAccount,
      action: toOperatorManagementActionArgs(parsedAction.action),
    });

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(operatorSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(manageOperatorInstruction, m),
    );

    const transaction =
      await signTransactionMessageWithSigners(transactionMessage);

    assertIsSendableTransaction(transaction);
    assertIsTransactionWithBlockhashLifetime(transaction);

    const sendAndConfirmTransaction = sendAndConfirmTransactionFactory({
      rpc: this.rpc,
      rpcSubscriptions: this.rpcSubscriptions,
    });

    try {
      await sendAndConfirmTransaction(transaction, {
        commitment: "confirmed",
      });
      const signature = getSignatureFromTransaction(transaction);
      this.logger.info(`Transaction successful: ${signature}`);
    } catch (error) {
      this.logger.error(`Update operator failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }

  private parseActionFlags(
    flags: Interfaces.InferredFlags<typeof UpdateOperator.flags>,
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
