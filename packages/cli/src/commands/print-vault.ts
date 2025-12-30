import { Flags } from "@oclif/core";
import { ReadonlyUint8Array } from "@solana/kit";
import BaseCommand from "../base-command";
import {
  fetchVault,
  VaultStatus,
  type OracleType,
  type PeriodLimit,
} from "jup-stable-sdk";
import { parseAddressFlag } from "../utils/common";
import { findVaultTokenAccount, findVault } from "jup-stable-sdk";

type VaultAccount = Awaited<ReturnType<typeof fetchVault>>;

const ORACLE_PRICE_DECIMALS = 4;
export default class PrintVault extends BaseCommand {
  static summary = "Fetch and display a Jupiter Stable Vault account.";

  static description = `
This command derives the vault PDA for the provided mint and prints its contents.
`;

  static flags = {
    ...BaseCommand.flags,
    mint: Flags.string({
      description: "Base58 address of the vault's mint.",
      required: true,
      parse: async (input) => input.trim(),
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(PrintVault);

    this.configureRpcClients();

    const mint = parseAddressFlag(flags.mint, "mint");

    const vaultAddress = await findVault(mint);
    const vaultTokenAccountAddress = await findVaultTokenAccount(mint);

    this.logger.info(`Fetching vault for mint ${mint}...`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Vault token account PDA: ${vaultTokenAccountAddress}`);

    let vaultAccount: VaultAccount;
    try {
      vaultAccount = await fetchVault(this.rpc, vaultAddress);
    } catch (error) {
      this.logger.error(
        `Failed to fetch vault account: ${(error as Error).message ?? "unknown error"}`,
      );
      process.exit(1);
    }

    const data = vaultAccount.data;
    const statusName = VaultStatus[data.status] ?? "Unknown";

    this.logger.info("Vault account data:");
    this.logger.info(`  Mint: ${data.mint}`);
    this.logger.info(`  Custodian: ${data.custodian}`);
    this.logger.info(`  Token account: ${data.tokenAccount}`);
    this.logger.info(`  Token program: ${data.tokenProgram}`);
    this.logger.info(`  Status: ${statusName}`);
    this.logger.info(
      `  Staleness threshold (seconds): ${data.stalesnessThreshold.toString()}`,
    );
    this.logger.info(
      `  Min oracle price USD (scaled ${ORACLE_PRICE_DECIMALS}dp): ${data.minOraclePriceUsd.toString()}`,
    );
    this.logger.info(
      `  Max oracle price USD (scaled ${ORACLE_PRICE_DECIMALS}dp): ${data.maxOraclePriceUsd.toString()}`,
    );
    this.logger.info(`  Bump: ${data.bump}`);
    this.logger.info(`  Decimals: ${data.decimals}`);
    this.logger.info(`  Total minted: ${formatU128Le(data.totalMinted)}`);
    this.logger.info(`  Total redeemed: ${formatU128Le(data.totalRedeemed)}`);

    this.logOracles(data.oracles);
    this.logPeriodLimits(data.periodLimits);
  }

  private logOracles(oracles: OracleType[]): void {
    const active = oracles
      .map((oracle, index) => ({ oracle, index }))
      .filter(({ oracle }) => oracle.__kind !== "Empty");

    if (active.length === 0) {
      this.logger.info("  Oracles: none configured");
      return;
    }

    this.logger.info("  Oracles:");
    for (const { oracle, index } of active) {
      this.logger.info(`    [${index}] ${oracle.__kind}`);

      switch (oracle.__kind) {
        case "Pyth": {
          const feedId = Buffer.from(oracle.fields[0].feedId).toString("hex");
          this.logger.info(`      feed_id: ${feedId}`);
          this.logger.info(`      account: ${oracle.fields[0].account}`);
          break;
        }
        case "SwitchboardOnDemand": {
          this.logger.info(`      account: ${oracle.fields[0].account}`);
          break;
        }
        case "Doves": {
          this.logger.info(`      account: ${oracle.fields[0].account}`);
          break;
        }
        default:
          break;
      }
    }
  }

  private logPeriodLimits(periodLimits: PeriodLimit[]): void {
    this.logger.info("  Period limits:");
    periodLimits.forEach((limit, index) => {
      this.logger.info(`    [${index}]`);
      this.logger.info(
        `      duration_seconds: ${limit.durationSeconds.toString()}`,
      );
      this.logger.info(
        `      max_mint_amount: ${limit.maxMintAmount.toString()}`,
      );
      this.logger.info(
        `      max_redeem_amount: ${limit.maxRedeemAmount.toString()}`,
      );
      this.logger.info(`      minted_amount: ${limit.mintedAmount.toString()}`);
      this.logger.info(
        `      redeemed_amount: ${limit.redeemedAmount.toString()}`,
      );
      this.logger.info(`      window_start: ${limit.windowStart.toString()}`);
    });
  }
}

function formatU128Le(value: ReadonlyUint8Array): string {
  let result = 0n;
  for (let i = 0; i < value.length; i++) {
    result += BigInt(value[i]) << (BigInt(i) * 8n);
  }
  return result.toString();
}
