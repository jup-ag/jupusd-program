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
import { parseAddressFlag } from "../utils/common";
import { findBenefactor, findOperator } from "jup-stable-sdk";
import { getDeleteBenefactorInstruction } from "jup-stable-sdk";
import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

export default class DeleteBenefactor extends BaseCommand {
  static summary = "Delete an existing benefactor PDA.";

  static description = `
This command closes an existing benefactor PDA and returns its rent to the specified receiver account.
The invoking signer must be an enabled operator with the Benefactor Manager role.
`;

  static flags = {
    ...BaseCommand.flags,
    "benefactor-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the benefactor PDA to delete.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    receiver: Flags.string({
      description:
        "Optional base58 address that will receive the reclaimed rent (defaults to the operator authority).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(DeleteBenefactor);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const benefactorAuthority = parseAddressFlag(
      flags["benefactor-authority"],
      "benefactor-authority",
    );
    const receiverAddress: Address =
      flags.receiver !== undefined
        ? parseAddressFlag(flags.receiver, "receiver")
        : (operatorSigner.address as Address);

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    this.logger.info("Deleting benefactor with:");
    this.logger.info(`  Benefactor PDA: ${benefactorAccount}`);
    this.logger.info(`  Rent receiver: ${receiverAddress}`);

    const instruction = getDeleteBenefactorInstruction({
      operatorAuthority: operatorSigner,
      operator: operatorAccount,
      receiver: receiverAddress,
      benefactor: benefactorAccount,
    });

    const confirmationInterface = createInterface({ input, output });
    let answer = "";

    try {
      await this.logger.flush();
      answer = await confirmationInterface.question(
        "Continue deleting the benefactor? (y/n) ",
      );
    } finally {
      confirmationInterface.close();
    }

    if (!/^(y|yes)$/i.test(answer.trim())) {
      this.logger.info("Deletion cancelled.");
      return;
    }

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(operatorSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(instruction, m),
    );

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
        `Delete benefactor failed: ${(error as Error).message}`,
      );
      process.exit(1);
    }
  }
}
