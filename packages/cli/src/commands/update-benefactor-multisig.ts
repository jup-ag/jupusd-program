import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import bs58 from "bs58";
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
  getManageBenefactorInstruction,
  type BenefactorManagementActionArgs,
} from "jupusd-sdk";
import { parseAddressFlag, parseU64StringFlag } from "../utils/common";
import {
  parseBenefactorFeeRateFlag,
  parseBenefactorStatusFlag,
} from "../utils/benefactor";
import { findBenefactor, findOperator } from "jupusd-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

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
  summary: string;
  details: Record<string, string>;
};

export default class UpdateBenefactorMultisig extends BaseCommand {
  static summary = "Queue benefactor management actions via Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that updates an existing benefactor PDA using the selected management actions.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
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
    const { flags } = await this.parse(UpdateBenefactorMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
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
        operatorAuthority: multisigAuthority,
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
