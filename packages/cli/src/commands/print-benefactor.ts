import { Flags } from "@oclif/core";
import { ReadonlyUint8Array } from "@solana/kit";
import BaseCommand from "../base-command";
import {
  fetchBenefactor,
  BenefactorStatus,
  type PeriodLimit,
} from "jup-stable-sdk";
import { parseAddressFlag } from "../utils/common";
import { findBenefactor } from "jup-stable-sdk";

type BenefactorAccount = Awaited<ReturnType<typeof fetchBenefactor>>;

export default class PrintBenefactor extends BaseCommand {
  static summary = "Fetch and display a Jupiter Stable Benefactor account.";

  static description = `
This command derives the benefactor PDA for the provided authority and prints its contents.
`;

  static flags = {
    ...BaseCommand.flags,
    authority: Flags.string({
      description: "Base58 address of the benefactor authority.",
      required: true,
      parse: async (input) => input.trim(),
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(PrintBenefactor);

    this.configureRpcClients();

    const authority = parseAddressFlag(flags.authority, "authority");

    const benefactorAddress = await findBenefactor(authority);

    this.logger.info(`Fetching benefactor for authority ${authority}...`);
    this.logger.info(`  Benefactor PDA: ${benefactorAddress}`);

    let benefactorAccount: BenefactorAccount;
    try {
      benefactorAccount = await fetchBenefactor(this.rpc, benefactorAddress);
    } catch (error) {
      this.logger.error(
        `Failed to fetch benefactor account: ${(error as Error).message ?? "unknown error"}`,
      );
      process.exit(1);
    }

    const data = benefactorAccount.data;
    const statusDescription = formatStatus(data.status);

    this.logger.info("Benefactor account data:");
    this.logger.info(`  Authority: ${data.authority}`);
    this.logger.info(`  Status: ${statusDescription}`);
    this.logger.info(`  Mint fee rate (bps): ${data.mintFeeRate}`);
    this.logger.info(`  Redeem fee rate (bps): ${data.redeemFeeRate}`);
    this.logger.info(`  Total minted: ${formatU128Le(data.totalMinted)}`);
    this.logger.info(`  Total redeemed: ${formatU128Le(data.totalRedeemed)}`);

    this.logPeriodLimits(data.periodLimits);
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

function formatStatus(status: BenefactorStatus): string {
  switch (status) {
    case BenefactorStatus.Active:
      return "active";
    case BenefactorStatus.Disabled:
      return "disabled";
    default:
      return "unknown";
  }
}

function formatU128Le(value: ReadonlyUint8Array): string {
  let result = 0n;
  for (let i = 0; i < value.length; i++) {
    result += BigInt(value[i]) << (BigInt(i) * 8n);
  }
  return result.toString();
}
