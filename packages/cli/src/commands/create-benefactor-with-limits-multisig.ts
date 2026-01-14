import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import bs58 from "bs58";
import { parseAddressFlag, parseU64StringFlag } from "../utils/common";
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
  getCreateBenefactorInstructionAsync,
  getManageBenefactorInstruction,
  findBenefactor,
  findOperator,
  BenefactorStatus,
} from "jupusd-sdk";
import { parseBenefactorFeeRateFlag } from "../utils/benefactor";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class CreateBenefactorWithLimitsMultisig extends BaseCommand {
  static summary =
    "Create a new benefactor PDA with fees, limits and enable it using Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that creates a new benefactor PDA with the provided fee rates, period limits, and enables the benefactor in a single transaction.

Period limits are configured by index:
  - Index 0: Hourly limit (3600 seconds)
  - Index 1: Daily limit (86400 seconds)
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
        "Base58 address of the authority that controls the benefactor PDA.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "mint-fee-rate": Flags.integer({
      description: "Mint fee rate in basis points (0-10000).",
      required: false,
      default: 4,
      min: 0,
      max: 10000,
    }),
    "redeem-fee-rate": Flags.integer({
      description: "Redeem fee rate in basis points (0-10000).",
      required: false,
      default: 4,
      min: 0,
      max: 10000,
    }),
    "hourly-max-mint": Flags.string({
      description:
        "Maximum mint amount (raw units) per hour. Default: 10,000,000 tokens (10,000,000,000,000 raw units with 6 decimals).",
      required: false,
      default: "10000000000000",
      parse: async (input) => input.trim(),
    }),
    "hourly-max-redeem": Flags.string({
      description:
        "Maximum redeem amount (raw units) per hour. Default: 10,000,000 tokens (10,000,000,000,000 raw units with 6 decimals).",
      required: false,
      default: "10000000000000",
      parse: async (input) => input.trim(),
    }),
    "daily-max-mint": Flags.string({
      description:
        "Maximum mint amount (raw units) per day. Default: 100,000,000 tokens (100,000,000,000,000 raw units with 6 decimals).",
      required: false,
      default: "100000000000000",
      parse: async (input) => input.trim(),
    }),
    "daily-max-redeem": Flags.string({
      description:
        "Maximum redeem amount (raw units) per day. Default: 100,000,000 tokens (100,000,000,000,000 raw units with 6 decimals).",
      required: false,
      default: "100000000000000",
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateBenefactorWithLimitsMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const mintFeeRate = parseBenefactorFeeRateFlag(
      flags["mint-fee-rate"],
      "mint-fee-rate",
    );
    const redeemFeeRate = parseBenefactorFeeRateFlag(
      flags["redeem-fee-rate"],
      "redeem-fee-rate",
    );

    const benefactorAuthority = parseAddressFlag(
      flags["benefactor-authority"],
      "benefactor-authority",
    );

    const operatorAccount = await findOperator(multisigAuthority.address);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    this.logger.info("Creating benefactor with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Benefactor authority: ${benefactorAuthority}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAccount}`);
    this.logger.info(`  Mint fee rate: ${mintFeeRate} bps`);
    this.logger.info(`  Redeem fee rate: ${redeemFeeRate} bps`);

    // Build list of instructions
    const instructions = [];

    // 1. Create benefactor instruction
    const createInstruction = await getCreateBenefactorInstructionAsync({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      payer: multisigAuthority,
      benefactorAuthority,
      benefactor: benefactorAccount,
      mintFeeRate,
      redeemFeeRate,
    });
    instructions.push(createInstruction);

    // 2. Add hourly limit instruction (always applied with defaults)
    {
      const hourlyMaxMint = parseU64StringFlag(
        flags["hourly-max-mint"],
        "hourly-max-mint",
      );
      const hourlyMaxRedeem = parseU64StringFlag(
        flags["hourly-max-redeem"],
        "hourly-max-redeem",
      );

      this.logger.info("  Hourly limits:");
      this.logger.info(`    Max mint: ${hourlyMaxMint.toString()}`);
      this.logger.info(`    Max redeem: ${hourlyMaxRedeem.toString()}`);

      const hourlyInstruction = getManageBenefactorInstruction({
        operatorAuthority: multisigAuthority,
        operator: operatorAccount,
        benefactor: benefactorAccount,
        action: {
          __kind: "UpdatePeriodLimit",
          index: 0, // Hourly limit at index 0
          durationSeconds: 3600n, // 1 hour
          maxMintAmount: hourlyMaxMint,
          maxRedeemAmount: hourlyMaxRedeem,
        },
      });
      instructions.push(hourlyInstruction);
    }

    // 3. Add daily limit instruction (always applied with defaults)
    {
      const dailyMaxMint = parseU64StringFlag(
        flags["daily-max-mint"],
        "daily-max-mint",
      );
      const dailyMaxRedeem = parseU64StringFlag(
        flags["daily-max-redeem"],
        "daily-max-redeem",
      );

      this.logger.info("  Daily limits:");
      this.logger.info(`    Max mint: ${dailyMaxMint.toString()}`);
      this.logger.info(`    Max redeem: ${dailyMaxRedeem.toString()}`);

      const dailyInstruction = getManageBenefactorInstruction({
        operatorAuthority: multisigAuthority,
        operator: operatorAccount,
        benefactor: benefactorAccount,
        action: {
          __kind: "UpdatePeriodLimit",
          index: 1, // Daily limit at index 1
          durationSeconds: 86400n, // 24 hours
          maxMintAmount: dailyMaxMint,
          maxRedeemAmount: dailyMaxRedeem,
        },
      });
      instructions.push(dailyInstruction);
    }

    // 4. Enable the benefactor (set status to Active)
    this.logger.info("  Status: Active");
    const enableInstruction = getManageBenefactorInstruction({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      benefactor: benefactorAccount,
      action: {
        __kind: "SetStatus",
        status: BenefactorStatus.Active,
      },
    });
    instructions.push(enableInstruction);

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    let transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    );

    // Append all instructions to the transaction
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
}
