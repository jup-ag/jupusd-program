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
import { fetchConfig, getCreateVaultInstructionAsync } from "jup-stable-sdk";
import {
  findConfig,
  findOperator,
  findVaultTokenAccount,
  findVault,
} from "jup-stable-sdk";

type CreateVaultFlagInput = Interfaces.InferredFlags<typeof CreateVault.flags>;

export default class CreateVault extends BaseCommand {
  static summary = "Create a new vault account for a collateral mint.";

  static description = `
This command creates the vault PDA and its associated token account for the specified collateral mint.
The invoking signer must be an enabled operator with the Vault Manager role.
`;

  static flags = {
    ...BaseCommand.flags,
    mint: Flags.string({
      description: "Base58 address of the vault's mint.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "payer-keypair-file": Flags.string({
      description:
        "Optional path to a keypair file that will fund the vault accounts (defaults to the invoking operator authority).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateVault);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const payerSigner = await this.resolvePayerSigner(flags, operatorSigner);
    assertIsTransactionSigner(payerSigner);

    const mint = parseAddressFlag(flags.mint, "mint");

    const operatorAuthorityAddress = operatorSigner.address as Address;

    const operatorAccount = await findOperator(operatorAuthorityAddress);
    const configAddress = await findConfig();

    let configAccount: Awaited<ReturnType<typeof fetchConfig>> | null = null;
    try {
      configAccount = await fetchConfig(this.rpc, configAddress as Address);
    } catch (error) {
      this.logger.error(
        `Failed to fetch config account at ${configAddress}: ${(error as Error).message}`,
      );
      process.exit(1);
    }

    if (!configAccount) {
      return;
    }

    const authorityAddress = configAccount.data.authority as Address;
    const tokenProgramAddress = configAccount.data.tokenProgram as Address;

    const vaultAddress = await findVault(mint);
    const vaultTokenAccountAddress = await findVaultTokenAccount(mint);

    this.logger.info("Creating vault with:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Collateral mint: ${mint}`);
    this.logger.info(`  Config PDA: ${configAddress}`);
    this.logger.info(`  Config authority: ${authorityAddress}`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Vault token account: ${vaultTokenAccountAddress}`);
    this.logger.info(`  Payer: ${payerSigner.address}`);

    const instruction = await getCreateVaultInstructionAsync({
      operatorAuthority: operatorSigner,
      operator: operatorAccount,
      payer: payerSigner,
      mint,
      config: configAddress,
      authority: authorityAddress,
      tokenProgram: tokenProgramAddress,
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
      this.logger.error(`Create vault failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }

  private async resolvePayerSigner(
    flags: CreateVaultFlagInput,
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
