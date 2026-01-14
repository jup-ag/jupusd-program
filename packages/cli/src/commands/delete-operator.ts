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
import { findOperator, getDeleteOperatorInstruction } from "jupusd-sdk";
import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

type DeleteOperatorFlagInput = Interfaces.InferredFlags<
  typeof DeleteOperator.flags
>;

export default class DeleteOperator extends BaseCommand {
  static summary = "Delete an existing operator PDA.";

  static description = `
This command closes an existing operator PDA and returns its rent to the payer signer.
The invoking signer must be an enabled operator with the Admin role.
`;

  static flags = {
    ...BaseCommand.flags,
    "operator-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the operator PDA to delete.",
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
    const { flags } = await this.parse(DeleteOperator);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const operatorAuthority = parseAddressFlag(
      flags["operator-authority"],
      "operator-authority",
    );
    const receiverAddress: Address =
      flags.receiver !== undefined
        ? parseAddressFlag(flags.receiver, "receiver")
        : (operatorAuthority as Address);

    const operatorAuthorityAddress = operatorSigner.address as Address;
    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const deletedOperatorAccount = await findOperator(operatorAuthority);

    this.logger.info("Deleting operator with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Deleted operator authority: ${operatorAuthority}`);
    this.logger.info(`  Deleted operator PDA: ${deletedOperatorAccount}`);
    this.logger.info(`  Rent receiver: ${receiverAddress}`);

    const instruction = getDeleteOperatorInstruction({
      operatorAuthority: operatorSigner,
      payer: operatorSigner,
      operator: operatorAccount,
      deletedOperator: deletedOperatorAccount,
    });

    const confirmationInterface = createInterface({ input, output });
    let answer = "";

    try {
      await this.logger.flush();
      answer = await confirmationInterface.question(
        "Continue deleting the operator? (y/n) ",
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
      this.logger.error(`Delete operator failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }
}
