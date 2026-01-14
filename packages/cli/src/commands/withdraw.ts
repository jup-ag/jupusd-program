import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import { parseAddressFlag, parseU64StringFlag } from "../utils/common";
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
import {
  fetchConfig,
  fetchVault,
  getWithdrawInstruction,
} from "jupusd-sdk";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { findConfig, findOperator, findVault } from "jupusd-sdk";

export default class Withdraw extends BaseCommand {
  static summary = "Withdraw collateral from a vault to its custodian.";

  static description = `
This command transfers collateral from a Jupiter Stable vault back to its designated custodian.
The invoking signer must be an enabled operator with the Vault Manager role.
`;

  static flags = {
    ...BaseCommand.flags,
    mint: Flags.string({
      description: "Base58 address of the vault's collateral mint.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    amount: Flags.string({
      description: "Amount of collateral (u64) to withdraw from the vault.",
      required: true,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(Withdraw);

    this.configureRpcClients();

    const operatorSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(operatorSigner);

    const mint = parseAddressFlag(flags.mint, "mint");
    const amount = parseU64StringFlag(flags.amount, "amount");
    if (amount === 0n) {
      this.error("Amount must be greater than zero.");
    }

    const operatorAuthorityAddress = operatorSigner.address as Address;
    const operatorAddress = await findOperator(operatorAuthorityAddress);
    const vaultAddress = await findVault(mint);
    const configAddress = await findConfig();

    let configAccount: Awaited<ReturnType<typeof fetchConfig>>;
    try {
      configAccount = await fetchConfig(this.rpc, configAddress as Address);
    } catch (error) {
      this.logger.error(
        `Failed to fetch config account at ${configAddress}: ${(error as Error).message}`,
      );
      process.exit(1);
    }

    let vaultAccount: Awaited<ReturnType<typeof fetchVault>>;
    try {
      vaultAccount = await fetchVault(this.rpc, vaultAddress as Address);
    } catch (error) {
      this.logger.error(
        `Failed to fetch vault account at ${vaultAddress}: ${(error as Error).message}`,
      );
      process.exit(1);
    }

    const authorityAddress = configAccount.data.authority as Address;
    const vaultTokenAccountAddress = vaultAccount.data.tokenAccount as Address;
    const vaultTokenProgramAddress = vaultAccount.data.tokenProgram as Address;
    const custodianAddress = vaultAccount.data.custodian as Address;

    const [custodianTokenAccount] = await findAssociatedTokenPda({
      mint,
      owner: custodianAddress,
      tokenProgram: vaultTokenProgramAddress,
    });

    let createCustodianAtaInstruction =
      await getCreateAssociatedTokenIdempotentInstructionAsync({
        mint,
        payer: operatorSigner,
        owner: custodianAddress,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
      });

    const withdrawInstruction = getWithdrawInstruction({
      operatorAuthority: operatorSigner,
      operator: operatorAddress,
      custodian: custodianAddress,
      custodianTokenAccount,
      config: configAddress,
      authority: authorityAddress,
      vault: vaultAddress,
      vaultTokenAccount: vaultTokenAccountAddress,
      vaultMint: mint,
      tokenProgram: vaultTokenProgramAddress,
      amount,
    });

    this.logger.info("Withdrawing collateral:");
    this.logger.info(`  Operator authority: ${operatorSigner.address}`);
    this.logger.info(`  Operator PDA: ${operatorAddress}`);
    this.logger.info(`  Vault mint: ${mint}`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Vault token account: ${vaultTokenAccountAddress}`);
    this.logger.info(`  Vault token program: ${vaultTokenProgramAddress}`);
    this.logger.info(`  Custodian authority: ${custodianAddress}`);
    this.logger.info(`  Custodian token account: ${custodianTokenAccount}`);
    this.logger.info(`  Amount: ${amount.toString()}`);

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(operatorSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) =>
        appendTransactionMessageInstruction(createCustodianAtaInstruction, m),
      (m) => appendTransactionMessageInstruction(withdrawInstruction, m),
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
      console.error(error);
      this.logger.error(
        `Withdraw failed: ${(error as Error).message ?? "unknown error"}`,
      );
      process.exit(1);
    }
  }
}
