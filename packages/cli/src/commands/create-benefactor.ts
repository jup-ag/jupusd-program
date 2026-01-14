import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import { parseKeypairFile, parseAddressFlag } from "../utils/common";
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
import { getCreateBenefactorInstructionAsync } from "jupusd-sdk";
import { parseBenefactorFeeRateFlag } from "../utils/benefactor";
import { findBenefactor, findOperator } from "jupusd-sdk";

type CreateBenefactorFlagInput = Interfaces.InferredFlags<
  typeof CreateBenefactor.flags
>;

export default class CreateBenefactor extends BaseCommand {
  static summary = "Create a new benefactor PDA.";

  static description = `
This command creates a new benefactor PDA for the supplied authority and initializes it with the provided fee rates.
The invoking signer must be an enabled operator with the Benefactor Manager role.
`;

  static flags = {
    ...BaseCommand.flags,
    "benefactor-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the benefactor PDA.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "mint-fee-rate": Flags.integer({
      description: "Mint fee rate in basis points (0-10000).",
      required: true,
      min: 0,
    }),
    "redeem-fee-rate": Flags.integer({
      description: "Redeem fee rate in basis points (0-10000).",
      required: true,
      min: 0,
    }),
    "payer-keypair-file": Flags.string({
      description:
        "Optional path to a keypair file that will fund the new benefactor PDA (defaults to the invoking operator authority).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateBenefactor);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const payerSigner = await this.resolvePayerSigner(flags, operatorSigner);
    assertIsTransactionSigner(payerSigner);

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

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    this.logger.info("Creating benefactor with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Benefactor authority: ${benefactorAuthority}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAccount}`);
    this.logger.info(`  Mint fee rate: ${mintFeeRate} bps`);
    this.logger.info(`  Redeem fee rate: ${redeemFeeRate} bps`);
    this.logger.info(`  Payer: ${payerSigner.address}`);

    const instruction = await getCreateBenefactorInstructionAsync({
      operatorAuthority: operatorSigner,
      operator: operatorAccount,
      payer: payerSigner,
      benefactorAuthority,
      benefactor: benefactorAccount,
      mintFeeRate,
      redeemFeeRate,
    });

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(payerSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(instruction, m),
    );

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
      this.logger.error(
        `Create benefactor failed: ${(error as Error).message}`,
      );
      process.exit(1);
    }
  }

  private async resolvePayerSigner(
    flags: CreateBenefactorFlagInput,
    operatorSigner: ReturnType<BaseCommand["getSolanaKeypair"]>,
  ) {
    const payerPath = flags["payer-keypair-file"];
    if (!payerPath) {
      return operatorSigner;
    }

    try {
      return await parseKeypairFile(payerPath);
    } catch (error) {
      this.error(
        `Failed to load payer keypair from ${payerPath}: ${(error as Error).message}`,
      );
    }
  }
}
