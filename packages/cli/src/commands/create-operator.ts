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
import { getCreateOperatorInstructionAsync } from "jupusd-sdk";
import { OPERATOR_ROLE_NAMES, parseOperatorRoleFlag } from "../utils/operator";
import { findOperator } from "jupusd-sdk";

type CreateOperatorFlagInput = Interfaces.InferredFlags<
  typeof CreateOperator.flags
>;

export default class CreateOperator extends BaseCommand {
  static summary = "Create a new operator account.";

  static description = `
This command creates a new operator PDA for the supplied authority and assigns it a role.
The current signer must already be an enabled operator with the Admin role.
`;

  static flags = {
    ...BaseCommand.flags,
    role: Flags.string({
      description: "Operator role to assign to the new operator.",
      required: true,
      options: [...OPERATOR_ROLE_NAMES],
    }),
    "new-operator-authority": Flags.string({
      description: "Base58 address of the authority for the new operator PDA.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "payer-keypair-file": Flags.string({
      description:
        "Optional path to a keypair file that will fund the new operator PDA (defaults to the invoking operator authority).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateOperator);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const payerSigner = await this.resolvePayerSigner(flags, operatorSigner);
    assertIsTransactionSigner(payerSigner);

    const role = parseOperatorRoleFlag(flags.role);
    const newOperatorAuthority = parseAddressFlag(
      flags["new-operator-authority"],
      "new-operator-authority",
    );

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const newOperatorAccount = await findOperator(newOperatorAuthority);

    this.logger.info("Creating operator with:");
    this.logger.info(
      `  Invoking operator authority: ${operatorSigner.address}`,
    );
    this.logger.info(`  Existing operator PDA: ${operatorAccount}`);
    this.logger.info(`  New operator authority: ${newOperatorAuthority}`);
    this.logger.info(`  New operator PDA: ${newOperatorAccount}`);
    this.logger.info(`  Role: ${role.name}`);
    this.logger.info(`  Payer: ${payerSigner.address}`);

    assertIsTransactionSigner(operatorSigner);
    assertIsTransactionSigner(payerSigner);

    const instruction = await getCreateOperatorInstructionAsync({
      operatorAuthority: operatorSigner,
      payer: payerSigner,
      operator: operatorAccount,
      newOperatorAuthority,
      newOperator: newOperatorAccount,
      role: role.role,
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
      this.logger.error(`Create operator failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }

  private async resolvePayerSigner(
    flags: CreateOperatorFlagInput,
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
