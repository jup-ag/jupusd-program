import BaseCommand from "../base-command";
import { fetchConfig, type PeriodLimit } from "jup-stable-sdk";
import { findConfig } from "jup-stable-sdk";

const PEG_PRICE_DECIMALS = 4;
type ConfigAccount = Awaited<ReturnType<typeof fetchConfig>>;

export default class PrintConfig extends BaseCommand {
  static summary = "Fetch and display the Jupiter Stable Config account.";

  static description = `
This command fetches the Config PDA and prints its contents.
`;

  static flags = {
    ...BaseCommand.flags,
  };

  async run(): Promise<void> {
    await this.parse(PrintConfig);

    this.configureRpcClients();

    const configAddress = await findConfig();

    this.logger.info(`Fetching config at ${configAddress}...`);

    let configAccount: ConfigAccount;
    try {
      configAccount = await fetchConfig(this.rpc, configAddress);
    } catch (error) {
      this.logger.error(
        `Failed to fetch config: ${(error as Error).message ?? "unknown error"}`,
      );
      process.exit(1);
    }
    const data = configAccount.data;

    this.logger.info("Config account data:");
    this.logger.info(`  Address: ${configAddress}`);
    this.logger.info(`  Mint: ${data.mint}`);
    this.logger.info(`  Authority: ${data.authority}`);
    this.logger.info(`  Token program: ${data.tokenProgram}`);
    this.logger.info(
      `  Peg price (display): ${formatPegPrice(data.pegPriceUsd)} (raw: ${data.pegPriceUsd.toString()})`,
    );
    this.logger.info(
      `  Mint/redeem enabled: ${data.isMintRedeemEnabled !== 0 ? "true" : "false"}`,
    );
    this.logger.info(`  Authority bump: ${data.authorityBump}`);
    this.logger.info(`  Config bump: ${data.configBump}`);

    this.logger.info("  Period limits:");
    data.periodLimits.forEach((limit, index) => {
      this.logPeriodLimit(limit, index);
    });
  }

  private logPeriodLimit(limit: PeriodLimit, index: number): void {
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
  }
}

function formatPegPrice(value: bigint): string {
  const scalingFactor = 10n ** BigInt(PEG_PRICE_DECIMALS);
  const whole = value / scalingFactor;
  const fraction = value % scalingFactor;

  if (fraction === 0n) {
    return whole.toString();
  }

  const fractionString = fraction
    .toString()
    .padStart(PEG_PRICE_DECIMALS, "0")
    .replace(/0+$/, "");

  return fractionString.length > 0
    ? `${whole.toString()}.${fractionString}`
    : whole.toString();
}
