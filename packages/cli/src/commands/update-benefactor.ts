import { Flags, Interfaces } from "@oclif/core";
import {
  Address,
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
import BaseCommand from "../base-command";
import {
  getManageBenefactorInstruction,
  type BenefactorManagementActionArgs,
} from "jupusd-sdk";
import { parseAddressFlag } from "../utils/common";
import {
  parseBenefactorFeeRateFlag,
  parseBenefactorStatusFlag,
} from "../utils/benefactor";
import { parseU64StringFlag } from "../utils/common";
import { findBenefactor, findOperator } from "jupusd-sdk";

const BENEFACTOR_ACTION_OPTIONS = [
  "disable",
  "set-status",
  "update-fee-rates",
  "update-period-limit",
  "reset-period-limit",
] as const;

type BenefactorActionOption = (typeof BENEFACTOR_ACTION_OPTIONS)[number];

type UpdateBenefactorFlagInput = Interfaces.InferredFlags<
  typeof UpdateBenefactor.flags
>;

type ParsedActionResult = {
  action: BenefactorManagementActionArgs;
  summary: string;
  details: Record<string, string>;
};

export default class UpdateBenefactor extends BaseCommand {
  static summary = "Manage an existing benefactor PDA.";

  static description = `
This command invokes the manage_benefactor instruction to modify the status, fee rates, or period limits of an existing benefactor PDA.
The invoking signer must be an enabled operator with the appropriate role for the selected action.
`;

  static flags = {
    ...BaseCommand.flags,
    "benefactor-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the benefactor PDA to manage.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    action: Flags.string({
      description:
        "Benefactor management action to perform. Provide multiple times to queue several actions.",
      required: true,
      multiple: true,
      options: [...BENEFACTOR_ACTION_OPTIONS],
    }),
    status: Flags.string({
      description:
        "Desired status for the benefactor (set-status action). Accepts values such as active/disabled, enabled/disabled, or true/false.",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "mint-fee-rate": Flags.integer({
      description: "Mint fee rate in basis points (update-fee-rates action).",
      required: false,
      min: 0,
      max: 10000,
    }),
    "redeem-fee-rate": Flags.integer({
      description: "Redeem fee rate in basis points (update-fee-rates action).",
      required: false,
      min: 0,
      max: 10000,
    }),
    index: Flags.integer({
      description:
        "Index used by update-period-limit and reset-period-limit actions.",
      required: false,
      min: 0,
    }),
    "duration-seconds": Flags.string({
      description:
        "Rolling window duration in seconds (update-period-limit action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "max-mint-amount": Flags.string({
      description:
        "Maximum mint amount (raw units) for the specified period limit (update-period-limit action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "max-redeem-amount": Flags.string({
      description:
        "Maximum redeem amount (raw units) for the specified period limit (update-period-limit action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(UpdateBenefactor);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const benefactorAuthority = parseAddressFlag(
      flags["benefactor-authority"],
      "benefactor-authority",
    );

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    const actions = (flags.action ?? []) as BenefactorActionOption[];
    if (actions.length === 0) {
      this.error("At least one --action must be provided.");
    }

    const parsedActions = actions.map((action, index) =>
      this.parseAction(action, flags, index),
    );

    this.logger.info("Updating benefactor with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Benefactor authority: ${benefactorAuthority}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAccount}`);
    this.logger.info("  Actions:");
    parsedActions.forEach((parsedAction, idx) => {
      this.logger.info(`    ${idx + 1}. ${parsedAction.summary}`);
      for (const [key, value] of Object.entries(parsedAction.details)) {
        this.logger.info(`       ${key}: ${value}`);
      }
    });

    const instructions = parsedActions.map((parsedAction) =>
      getManageBenefactorInstruction({
        operatorAuthority: operatorSigner,
        operator: operatorAccount,
        benefactor: benefactorAccount,
        action: parsedAction.action,
      }),
    );

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    let transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(operatorSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    );

    for (const instruction of instructions) {
      transactionMessage = appendTransactionMessageInstruction(
        instruction,
        transactionMessage,
      ) as unknown as typeof transactionMessage;
    }

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
      this.logger.error(
        `Update benefactor failed: ${(error as Error).message}`,
      );
      process.exit(1);
    }
  }

  private parseAction(
    action: BenefactorActionOption,
    flags: UpdateBenefactorFlagInput,
    index: number,
  ): ParsedActionResult {
    switch (action) {
      case "disable":
        return {
          action: { __kind: "Disable" },
          summary: "Disable benefactor",
          details: {},
        };

      case "set-status": {
        const statusRaw = flags.status;
        if (statusRaw === undefined) {
          this.error("--status is required when action is set-status");
        }

        const parsed = parseBenefactorStatusFlag(statusRaw, "status");
        return {
          action: { __kind: "SetStatus", status: parsed.status },
          summary: `Set benefactor status to ${parsed.name}`,
          details: { status: parsed.name },
        };
      }

      case "update-fee-rates": {
        const mintFeeRate = parseBenefactorFeeRateFlag(
          flags["mint-fee-rate"],
          "mint-fee-rate",
        );
        const redeemFeeRate = parseBenefactorFeeRateFlag(
          flags["redeem-fee-rate"],
          "redeem-fee-rate",
        );

        return {
          action: {
            __kind: "UpdateFeeRates",
            mintFeeRate,
            redeemFeeRate,
          },
          summary: "Update benefactor fee rates",
          details: {
            "mint-fee-rate": `${mintFeeRate} bps`,
            "redeem-fee-rate": `${redeemFeeRate} bps`,
          },
        };
      }

      case "update-period-limit": {
        const indexValue = flags.index;
        const durationRaw = flags["duration-seconds"];
        const maxMintRaw = flags["max-mint-amount"];
        const maxRedeemRaw = flags["max-redeem-amount"];

        if (indexValue === undefined) {
          this.error("--index is required when action is update-period-limit");
        }
        if (durationRaw === undefined) {
          this.error(
            "--duration-seconds is required when action is update-period-limit",
          );
        }
        if (maxMintRaw === undefined) {
          this.error(
            "--max-mint-amount is required when action is update-period-limit",
          );
        }
        if (maxRedeemRaw === undefined) {
          this.error(
            "--max-redeem-amount is required when action is update-period-limit",
          );
        }

        const durationSeconds = parseU64StringFlag(
          durationRaw,
          "duration-seconds",
        );
        const maxMintAmount = parseU64StringFlag(maxMintRaw, "max-mint-amount");
        const maxRedeemAmount = parseU64StringFlag(
          maxRedeemRaw,
          "max-redeem-amount",
        );

        return {
          action: {
            __kind: "UpdatePeriodLimit",
            index: indexValue,
            durationSeconds,
            maxMintAmount,
            maxRedeemAmount,
          },
          summary: `Update period limit at index ${indexValue}`,
          details: {
            index: String(indexValue),
            "duration-seconds": durationSeconds.toString(),
            "max-mint-amount": maxMintAmount.toString(),
            "max-redeem-amount": maxRedeemAmount.toString(),
          },
        };
      }

      case "reset-period-limit": {
        const indexValue = flags.index;
        if (indexValue === undefined) {
          this.error("--index is required when action is reset-period-limit");
        }

        return {
          action: { __kind: "ResetPeriodLimit", index: indexValue },
          summary: `Reset period limit at index ${indexValue}`,
          details: { index: String(indexValue) },
        };
      }

      default:
        this.error(`Unsupported action (${index + 1}): ${action}`);
    }
  }
}
