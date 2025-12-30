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
  type Address,
} from "@solana/kit";
import BaseCommand from "../base-command";
import {
  getManageVaultInstruction,
  VaultStatus,
  oracleConfig,
  vaultManagementAction,
  type OracleConfigArgs,
  type VaultManagementActionArgs,
} from "jup-stable-sdk";
import { parseAddressFlag } from "../utils/common";
import { parseBooleanFlag, parseU64StringFlag } from "../utils/common";
import { findOperator, findVault } from "jup-stable-sdk";

const VAULT_ACTION_OPTIONS = [
  "disable",
  "set-status",
  "update-oracle",
  "update-period-limit",
  "reset-period-limit",
  "set-custodian",
  "set-stalesness-threshold",
  "set-min-oracle-price",
  "set-max-oracle-price",
] as const;

type VaultActionOption = (typeof VAULT_ACTION_OPTIONS)[number];

const ORACLE_KIND_OPTIONS = [
  "none",
  "pyth",
  "switchboard-on-demand",
  "doves",
] as const;

type OracleKindFlag = (typeof ORACLE_KIND_OPTIONS)[number];
type UpdateVaultFlagInput = Interfaces.InferredFlags<typeof UpdateVault.flags>;
type ParsedActionResult = {
  action: VaultManagementActionArgs;
  summary: string;
  details: Record<string, string>;
};

export default class UpdateVault extends BaseCommand {
  static summary = "Manage an existing vault PDA.";

  static description = `
This command invokes the manage_vault instruction to update the configuration of an existing vault.
The invoking signer must be an enabled operator with the Vault Manager role.
`;

  static flags = {
    ...BaseCommand.flags,
    mint: Flags.string({
      description: "Base58 address of the vault's mint.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    action: Flags.string({
      description:
        "Vault management action to perform. Provide multiple times to queue several actions.",
      required: true,
      multiple: true,
      options: [...VAULT_ACTION_OPTIONS],
    }),
    status: Flags.string({
      description:
        "Target vault status (set-status action). Accepts boolean-like values such as enabled/disabled, true/false, etc.",
      required: false,
      parse: async (input) => input.trim(),
    }),
    index: Flags.integer({
      description:
        "Index used by update-oracle, update-period-limit, and reset-period-limit actions.",
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
    custodian: Flags.string({
      description:
        "Base58 address of the new custodian (set-custodian action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "stalesness-threshold": Flags.string({
      description:
        "Oracle staleness threshold in seconds (set-stalesness-threshold action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "min-oracle-price-usd": Flags.string({
      description:
        "Minimum oracle price in USD scaled by 1e4 (set-min-oracle-price action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "max-oracle-price-usd": Flags.string({
      description:
        "Maximum oracle price in USD scaled by 1e4 (set-max-oracle-price action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "oracle-kind": Flags.string({
      description: "Oracle configuration to apply (update-oracle action).",
      required: false,
      options: [...ORACLE_KIND_OPTIONS],
      parse: async (input) => input.trim().toLowerCase(),
    }),
    "pyth-feed-id": Flags.string({
      description:
        "Hex-encoded 32-byte feed identifier used with the Pyth oracle variant (update-oracle action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
    "oracle-address": Flags.string({
      description:
        "Base58 address of the oracle account used with Switchboard or Doves variants (update-oracle action).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(UpdateVault);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const mint = parseAddressFlag(flags.mint, "mint");

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const vaultAccount = await findVault(mint);

    const actions = (flags.action ?? []) as VaultActionOption[];
    if (actions.length === 0) {
      this.error("At least one --action must be provided.");
    }

    const parsedActions = actions.map((action, index) =>
      this.parseAction(action, flags, index),
    );

    this.logger.info("Updating vault with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Vault PDA: ${vaultAccount}`);
    this.logger.info(`  Collateral mint: ${mint}`);
    this.logger.info(`  Actions:`);
    parsedActions.forEach((parsedAction, idx) => {
      this.logger.info(`    ${idx + 1}. ${parsedAction.summary}`);
      for (const [key, value] of Object.entries(parsedAction.details)) {
        this.logger.info(`       ${key}: ${value}`);
      }
    });

    const instructions = parsedActions.map((parsedAction) =>
      getManageVaultInstruction({
        operatorAuthority: operatorSigner,
        operator: operatorAccount,
        vault: vaultAccount,
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

    assertIsTransactionWithBlockhashLifetime(transaction);
    assertIsSendableTransaction(transaction);

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
      this.logger.error(`Update vault failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }

  private parseAction(
    action: VaultActionOption,
    flags: UpdateVaultFlagInput,
    index: number,
  ): ParsedActionResult {
    switch (action) {
      case "disable":
        return {
          action: vaultManagementAction("Disable"),
          summary: "Disable vault",
          details: {},
        };

      case "set-status": {
        const statusRaw = flags.status;
        if (statusRaw === undefined) {
          this.error("--status is required when action is set-status");
        }

        const parsed = this.parseVaultStatus(statusRaw, "status");
        return {
          action: vaultManagementAction("SetStatus", {
            status: parsed.status,
          }),
          summary: `Set vault status to ${parsed.name}`,
          details: { status: parsed.name },
        };
      }

      case "update-oracle": {
        const actionIndex = flags.index;
        if (actionIndex === undefined) {
          this.error("--index is required when action is update-oracle");
        }

        const kindRaw = flags["oracle-kind"];
        if (!kindRaw) {
          this.error("--oracle-kind is required when action is update-oracle");
        }

        const parsedOracle = this.parseOracleConfig(
          kindRaw as OracleKindFlag,
          flags,
        );

        return {
          action: vaultManagementAction("UpdateOracle", {
            index: actionIndex,
            oracle: parsedOracle.config,
          }),
          summary: `Update oracle at index ${actionIndex}`,
          details: this.mergeDetails(
            {
              index: actionIndex.toString(),
              "oracle-kind": parsedOracle.kind,
            },
            parsedOracle.details,
          ),
        };
      }

      case "update-period-limit": {
        const actionIndex = flags.index;
        if (actionIndex === undefined) {
          this.error("--index is required when action is update-period-limit");
        }

        const durationRaw = flags["duration-seconds"];
        const maxMintRaw = flags["max-mint-amount"];
        const maxRedeemRaw = flags["max-redeem-amount"];

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
          action: vaultManagementAction("UpdatePeriodLimit", {
            index: actionIndex,
            durationSeconds,
            maxMintAmount,
            maxRedeemAmount,
          }),
          summary: `Update period limit at index ${actionIndex}`,
          details: {
            index: actionIndex.toString(),
            "duration-seconds": durationSeconds.toString(),
            "max-mint-amount": maxMintAmount.toString(),
            "max-redeem-amount": maxRedeemAmount.toString(),
          },
        };
      }

      case "reset-period-limit": {
        const actionIndex = flags.index;
        if (actionIndex === undefined) {
          this.error("--index is required when action is reset-period-limit");
        }

        return {
          action: vaultManagementAction("ResetPeriodLimit", {
            index: actionIndex,
          }),
          summary: `Reset period limit at index ${actionIndex}`,
          details: { index: actionIndex.toString() },
        };
      }

      case "set-custodian": {
        const custodianRaw = flags.custodian;
        if (!custodianRaw) {
          this.error("--custodian is required when action is set-custodian");
        }

        const custodian = parseAddressFlag(custodianRaw, "custodian");

        return {
          action: vaultManagementAction("SetCustodian", {
            newCustodian: custodian,
          }),
          summary: "Set vault custodian",
          details: { custodian },
        };
      }

      case "set-stalesness-threshold": {
        const thresholdRaw = flags["stalesness-threshold"];
        if (thresholdRaw === undefined) {
          this.error(
            "--stalesness-threshold is required when action is set-stalesness-threshold",
          );
        }

        const threshold = parseU64StringFlag(
          thresholdRaw,
          "stalesness-threshold",
        );

        return {
          action: vaultManagementAction("SetStalesnessThreshold", {
            stalesnessThreshold: threshold,
          }),
          summary: "Set stalesness threshold",
          details: { "stalesness-threshold": threshold.toString() },
        };
      }

      case "set-min-oracle-price": {
        const minRaw = flags["min-oracle-price-usd"];
        if (minRaw === undefined) {
          this.error(
            "--min-oracle-price-usd is required when action is set-min-oracle-price",
          );
        }

        const minPrice = parseU64StringFlag(minRaw, "min-oracle-price-usd");

        return {
          action: vaultManagementAction("SetMinOraclePrice", {
            minOraclePriceUsd: minPrice,
          }),
          summary: "Set minimum oracle price",
          details: { "min-oracle-price-usd": minPrice.toString() },
        };
      }

      case "set-max-oracle-price": {
        const maxRaw = flags["max-oracle-price-usd"];
        if (maxRaw === undefined) {
          this.error(
            "--max-oracle-price-usd is required when action is set-max-oracle-price",
          );
        }

        const maxPrice = parseU64StringFlag(maxRaw, "max-oracle-price-usd");

        return {
          action: vaultManagementAction("SetMaxOraclePrice", {
            maxOraclePriceUsd: maxPrice,
          }),
          summary: "Set maximum oracle price",
          details: { "max-oracle-price-usd": maxPrice.toString() },
        };
      }

      default:
        this.error(`Unsupported action (${index + 1}): ${action}`);
    }
  }

  private parseVaultStatus(raw: string, flagName: string) {
    const isEnabled = parseBooleanFlag(raw, flagName);
    return {
      name: isEnabled ? "enabled" : "disabled",
      status: isEnabled ? VaultStatus.Enabled : VaultStatus.Disabled,
    };
  }

  private mergeDetails(
    base: Record<string, string>,
    extra: Record<string, string>,
  ): Record<string, string> {
    const result: Record<string, string> = { ...base };
    for (const [key, value] of Object.entries(extra)) {
      result[key] = value;
    }
    return result;
  }

  private parseOracleConfig(
    kind: OracleKindFlag,
    flags: UpdateVaultFlagInput,
  ): {
    kind: OracleKindFlag;
    config: OracleConfigArgs;
    details: Record<string, string>;
  } {
    switch (kind) {
      case "none":
        return {
          kind,
          config: oracleConfig("None"),
          details: {},
        };

      case "pyth": {
        const oracleAddressRaw = flags["oracle-address"];
        if (!oracleAddressRaw) {
          this.error(
            "--oracle-address is required when oracle-kind is switchboard-on-demand",
          );
        }

        const oracleAddress = parseAddressFlag(
          oracleAddressRaw,
          "oracle-address",
        );

        const feedIdRaw = flags["pyth-feed-id"];
        if (!feedIdRaw) {
          this.error("--pyth-feed-id is required when oracle-kind is pyth");
        }

        const feedId = this.parsePythFeedId(feedIdRaw);
        return {
          kind,
          config: oracleConfig("Pyth", [feedId, oracleAddress]),
          details: { "pyth-feed-id": feedIdRaw },
        };
      }

      case "switchboard-on-demand": {
        const oracleAddressRaw = flags["oracle-address"];
        if (!oracleAddressRaw) {
          this.error(
            "--oracle-address is required when oracle-kind is switchboard-on-demand",
          );
        }

        const oracleAddress = parseAddressFlag(
          oracleAddressRaw,
          "oracle-address",
        );
        return {
          kind,
          config: oracleConfig("SwitchboardOnDemand", [oracleAddress]),
          details: { "oracle-address": oracleAddress },
        };
      }

      case "doves": {
        const oracleAddressRaw = flags["oracle-address"];
        if (!oracleAddressRaw) {
          this.error("--oracle-address is required when oracle-kind is doves");
        }

        const oracleAddress = parseAddressFlag(
          oracleAddressRaw,
          "oracle-address",
        );
        return {
          kind,
          config: oracleConfig("Doves", [oracleAddress]),
          details: { "oracle-address": oracleAddress },
        };
      }

      default:
        this.error(`Unsupported oracle kind: ${kind}`);
    }
  }

  private parsePythFeedId(value: string): Uint8Array {
    const normalized = value.toLowerCase().replace(/^0x/, "");
    if (!/^[0-9a-f]{64}$/.test(normalized)) {
      this.error(
        "--pyth-feed-id must be a 64-character hex string representing 32 bytes",
      );
    }

    const bytes = new Uint8Array(32);
    for (let i = 0; i < 32; i += 1) {
      const byte = normalized.slice(i * 2, i * 2 + 2);
      bytes[i] = Number.parseInt(byte, 16);
    }
    return bytes;
  }
}
