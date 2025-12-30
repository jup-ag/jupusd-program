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
import { getManageConfigInstruction } from "jup-stable-sdk";
import { parseBooleanFlag, parseU64StringFlag } from "../utils/common";
import { findConfig, findOperator } from "jup-stable-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

const PEG_PRICE_DECIMALS = 4;

type ConfigManagementActionInput =
  | { kind: "pause" }
  | { kind: "updatePauseFlag"; isMintRedeemEnabled: boolean }
  | {
      kind: "updatePeriodLimit";
      index: number;
      durationSeconds: bigint;
      maxMintAmount: bigint;
      maxRedeemAmount: bigint;
    }
  | { kind: "resetPeriodLimit"; index: number }
  | { kind: "setPegPriceUsd"; pegPriceUsd: bigint };

type ParsedActionResult = {
  action: ConfigManagementActionInput;
  summary: string;
  details: Record<string, string>;
};

function toConfigManagementActionArgs(action: ConfigManagementActionInput) {
  switch (action.kind) {
    case "pause":
      return { __kind: "Pause" } as const;
    case "updatePauseFlag":
      return {
        __kind: "UpdatePauseFlag",
        isMintRedeemEnabled: action.isMintRedeemEnabled,
      } as const;
    case "updatePeriodLimit":
      return {
        __kind: "UpdatePeriodLimit",
        index: action.index,
        durationSeconds: action.durationSeconds,
        maxMintAmount: action.maxMintAmount,
        maxRedeemAmount: action.maxRedeemAmount,
      } as const;
    case "resetPeriodLimit":
      return { __kind: "ResetPeriodLimit", index: action.index } as const;
    case "setPegPriceUsd":
      return {
        __kind: "SetPegPriceUSD",
        pegPriceUsd: action.pegPriceUsd,
      } as const;
    default:
      return assertNever(action);
  }
}

type UpdateConfigMultisigFlagInput = Interfaces.InferredFlags<
  typeof UpdateConfigMultisig.flags
>;

export default class UpdateConfigMultisig extends BaseCommand {
  static summary = "Queue a config update via Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that updates the Jupiter Stable program's Config account.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    action: Flags.string({
      description: "Config management action to perform.",
      required: true,
      options: [
        "pause",
        "update-pause-flag",
        "update-period-limit",
        "reset-period-limit",
        "set-peg-price-usd",
      ],
    }),
    "mint-redeem-enabled": Flags.string({
      description:
        "Target value for config.is_mint_redeem_enabled when using the update-pause-flag action (true/false).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    index: Flags.integer({
      description:
        "Period limit index (0-3) used by update-period-limit and reset-period-limit actions.",
      required: false,
      min: 0,
      max: 3,
    }),
    "duration-seconds": Flags.integer({
      description:
        "Rolling window duration in seconds for the specified period limit (update-period-limit action).",
      required: false,
      min: 0,
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
    "peg-price-usd": Flags.string({
      description:
        "Target USD peg price expressed as a decimal value (set-peg-price-usd action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(UpdateConfigMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const parsedAction = this.parseActionFlags(flags);

    const configAddress = await findConfig();
    const operatorAccount = await findOperator(multisigAuthority.address);

    this.logger.info("Updating config with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Config: ${configAddress}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Action: ${parsedAction.summary}`);
    if (Object.keys(parsedAction.details).length > 0) {
      for (const [key, value] of Object.entries(parsedAction.details)) {
        this.logger.info(`    ${key}: ${value}`);
      }
    }

    const manageConfigInstruction = getManageConfigInstruction({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      config: configAddress,
      action: toConfigManagementActionArgs(parsedAction.action),
    } as unknown as Parameters<typeof getManageConfigInstruction>[0]);

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(manageConfigInstruction, m),
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
    flags: UpdateConfigMultisigFlagInput,
  ): ParsedActionResult {
    const action = flags.action;

    switch (action) {
      case "pause":
        return {
          action: { kind: "pause" },
          summary: "Pause protocol (disable mint and redeem)",
          details: {},
        };

      case "update-pause-flag": {
        const rawValue = flags["mint-redeem-enabled"];
        if (rawValue === undefined) {
          this.error(
            "--mint-redeem-enabled is required when action is update-pause-flag",
          );
        }

        const isMintRedeemEnabled = parseBooleanFlag(
          rawValue,
          "mint-redeem-enabled",
        );
        return {
          action: { kind: "updatePauseFlag", isMintRedeemEnabled },
          summary: "Update mint/redeem pause flag",
          details: {
            mint_redeem_enabled: String(isMintRedeemEnabled),
          },
        };
      }

      case "update-period-limit": {
        const index = flags.index;
        const durationSeconds = flags["duration-seconds"];
        const maxMintAmountRaw = flags["max-mint-amount"];
        const maxRedeemAmountRaw = flags["max-redeem-amount"];

        if (
          index === undefined ||
          durationSeconds === undefined ||
          maxMintAmountRaw === undefined ||
          maxRedeemAmountRaw === undefined
        ) {
          this.error(
            "--index, --duration-seconds, --max-mint-amount and --max-redeem-amount are required when action is update-period-limit",
          );
        }

        if (durationSeconds <= 0) {
          this.error("--duration-seconds must be greater than 0");
        }

        const maxMintAmount = parseU64StringFlag(
          maxMintAmountRaw,
          "max-mint-amount",
        );
        const maxRedeemAmount = parseU64StringFlag(
          maxRedeemAmountRaw,
          "max-redeem-amount",
        );

        return {
          action: {
            kind: "updatePeriodLimit",
            index,
            durationSeconds: BigInt(durationSeconds),
            maxMintAmount,
            maxRedeemAmount,
          },
          summary: "Update period limit",
          details: {
            index: String(index),
            duration_seconds: String(durationSeconds),
            max_mint_amount: maxMintAmountRaw,
            max_redeem_amount: maxRedeemAmountRaw,
          },
        };
      }

      case "reset-period-limit": {
        const index = flags.index;
        if (index === undefined) {
          this.error("--index is required when action is reset-period-limit");
        }

        return {
          action: { kind: "resetPeriodLimit", index },
          summary: "Reset period limit counters",
          details: { index: String(index) },
        };
      }

      case "set-peg-price-usd": {
        const pegPriceRaw = flags["peg-price-usd"];
        if (!pegPriceRaw) {
          this.error(
            "--peg-price-usd is required when action is set-peg-price-usd",
          );
        }

        const pegPriceUsd = parsePegPriceFlag(pegPriceRaw);
        return {
          action: {
            kind: "setPegPriceUsd",
            pegPriceUsd: pegPriceUsd.scaledValue,
          },
          summary: "Set USD peg price",
          details: {
            peg_price_display: pegPriceUsd.displayValue,
            peg_price_raw: pegPriceUsd.scaledValue.toString(),
          },
        };
      }

      default:
        this.error(`Unsupported action: ${action}`);
    }
  }
}

function parsePegPriceFlag(value: string) {
  const trimmed = value.trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    throw new Error(
      `Invalid value for --peg-price-usd. Provide a positive decimal number (e.g. 1.0000). Received: ${value}`,
    );
  }

  const [whole, fraction = ""] = trimmed.split(".");
  if (fraction.length > PEG_PRICE_DECIMALS) {
    throw new Error(
      `Peg price precision cannot exceed ${PEG_PRICE_DECIMALS} decimal places. Received: ${value}`,
    );
  }

  const paddedFraction = fraction.padEnd(PEG_PRICE_DECIMALS, "0");
  const wholeValue = parseU64StringFlag(whole, "peg-price-usd");
  const fractionalValue = paddedFraction ? BigInt(paddedFraction) : 0n;
  const scalingFactor = 10n ** BigInt(PEG_PRICE_DECIMALS);
  const scaledValue = wholeValue * scalingFactor + fractionalValue;

  if (scaledValue <= 0n) {
    throw new Error("Peg price must be greater than 0");
  }

  const maxPegPrice = 2n * 10n ** BigInt(PEG_PRICE_DECIMALS);
  if (scaledValue >= maxPegPrice) {
    throw new Error(
      `Peg price must be less than ${Number(maxPegPrice) / 10 ** PEG_PRICE_DECIMALS}. Received: ${value}`,
    );
  }

  const displayFraction = paddedFraction.slice(0, PEG_PRICE_DECIMALS);
  const displayValue = `${whole}.${displayFraction}`.replace(/\.$/, "");

  return { scaledValue, displayValue };
}

function assertNever(_value: never): never {
  throw new Error("Unexpected action variant");
}
