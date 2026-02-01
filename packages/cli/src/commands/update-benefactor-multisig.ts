import {
  type BenefactorManagementActionArgs,
  getManageBenefactorInstruction,
} from "@jup-ag/jupusd-sdk";
import { findBenefactor, findOperator } from "@jup-ag/jupusd-sdk";
import { Flags, Interfaces } from "@oclif/core";
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
import { PublicKey, VersionedTransaction } from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import bs58 from "bs58";

import BaseCommand from "../base-command";
import {
  parseBenefactorFeeRateFlag,
  parseBenefactorStatusFlag,
} from "../utils/benefactor";
import { parseAddressFlag, parseU64StringFlag } from "../utils/common";

const BENEFACTOR_ACTION_OPTIONS = [
  "disable",
  "set-status",
  "update-fee-rates",
  "update-period-limit",
  "reset-period-limit",
] as const;

type BenefactorActionOption = (typeof BENEFACTOR_ACTION_OPTIONS)[number];

type UpdateBenefactorFlagInput = Interfaces.InferredFlags<
  typeof UpdateBenefactorMultisig.flags
>;

type ParsedActionResult = {
  action: BenefactorManagementActionArgs;
  details: Record<string, string>;
  summary: string;
};

export default class UpdateBenefactorMultisig extends BaseCommand {
  static summary = "Queue benefactor management actions via Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that updates an existing benefactor PDA using the selected management actions.
`;

  static flags = {
    ...BaseCommand.flags,
    action: Flags.string({
      description:
        "Benefactor management action to perform. Provide multiple times to queue several actions.",
      multiple: true,
      options: [...BENEFACTOR_ACTION_OPTIONS],
      required: true,
    }),
    "benefactor-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the benefactor PDA to manage.",
      parse: async (input) => input.trim(),
      required: true,
    }),
    "duration-seconds": Flags.string({
      description:
        "Rolling window duration in seconds (update-period-limit action).",
      parse: async (input) => input.trim(),
      required: false,
    }),
    index: Flags.integer({
      description:
        "Index used by update-period-limit and reset-period-limit actions.",
      min: 0,
      required: false,
    }),
    "max-mint-amount": Flags.string({
      description:
        "Maximum mint amount (raw units) for the specified period limit (update-period-limit action).",
      parse: async (input) => input.trim(),
      required: false,
    }),
    "max-redeem-amount": Flags.string({
      description:
        "Maximum redeem amount (raw units) for the specified period limit (update-period-limit action).",
      parse: async (input) => input.trim(),
      required: false,
    }),
    "mint-fee-rate": Flags.integer({
      description: "Mint fee rate in basis points (update-fee-rates action).",
      max: 10000,
      min: 0,
      required: false,
    }),
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      parse: async (input) => input.trim(),
      required: true,
    }),
    "redeem-fee-rate": Flags.integer({
      description: "Redeem fee rate in basis points (update-fee-rates action).",
      max: 10000,
      min: 0,
      required: false,
    }),
    status: Flags.string({
      description:
        "Desired status for the benefactor (set-status action). Accepts values such as active/disabled, enabled/disabled, or true/false.",
      parse: async (input) => input.trim(),
      required: false,
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(UpdateBenefactorMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      index: 0,
      multisigPda: multisigPublicKey,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const benefactorAuthority = parseAddressFlag(
      flags["benefactor-authority"],
      "benefactor-authority",
    );

    const operatorAccount = await findOperator(multisigAuthority.address);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    const actions = (flags.action ?? []) as BenefactorActionOption[];
    if (actions.length === 0) {
      this.error("At least one --action must be provided.");
    }

    const parsedActions = actions.map((action, index) =>
      this.parseAction(action, flags, index),
    );

    this.logger.info("Updating benefactor with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
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
        action: parsedAction.action,
        benefactor: benefactorAccount,
        operator: operatorAccount,
        operatorAuthority: multisigAuthority,
      }),
    );

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    let transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    );

    for (const instruction of instructions) {
      transactionMessage = appendTransactionMessageInstruction(
        instruction,
        transactionMessage,
      ) as unknown as typeof transactionMessage;
    }

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

  private parseAction(
    action: BenefactorActionOption,
    flags: UpdateBenefactorFlagInput,
    index: number,
  ): ParsedActionResult {
    switch (action) {
      case "disable":
        return {
          action: { __kind: "Disable" },
          details: {},
          summary: "Disable benefactor",
        };

      case "set-status": {
        const statusRaw = flags.status;
        if (statusRaw === undefined) {
          this.error("--status is required when action is set-status");
        }

        const parsed = parseBenefactorStatusFlag(statusRaw, "status");
        return {
          action: { __kind: "SetStatus", status: parsed.status },
          details: { status: parsed.name },
          summary: `Set benefactor status to ${parsed.name}`,
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
          details: {
            "mint-fee-rate": `${mintFeeRate} bps`,
            "redeem-fee-rate": `${redeemFeeRate} bps`,
          },
          summary: "Update benefactor fee rates",
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
            durationSeconds,
            index: indexValue,
            maxMintAmount,
            maxRedeemAmount,
          },
          details: {
            "duration-seconds": durationSeconds.toString(),
            index: String(indexValue),
            "max-mint-amount": maxMintAmount.toString(),
            "max-redeem-amount": maxRedeemAmount.toString(),
          },
          summary: `Update period limit at index ${indexValue}`,
        };
      }

      case "reset-period-limit": {
        const indexValue = flags.index;
        if (indexValue === undefined) {
          this.error("--index is required when action is reset-period-limit");
        }

        return {
          action: { __kind: "ResetPeriodLimit", index: indexValue },
          details: { index: String(indexValue) },
          summary: `Reset period limit at index ${indexValue}`,
        };
      }

      default:
        this.error(`Unsupported action (${index + 1}): ${action}`);
    }
  }
}
