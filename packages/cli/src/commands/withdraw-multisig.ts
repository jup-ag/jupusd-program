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
  type Address,
} from "@solana/kit";
import {
  fetchConfig,
  fetchVault,
  getWithdrawInstruction,
} from "jup-stable-sdk";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { findConfig, findOperator, findVault } from "jup-stable-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class WithdrawMultisig extends BaseCommand {
  static summary = "Queue a withdraw of vault collateral via Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that withdraws collateral from a Jupiter Stable vault into its custodian token account.
  `;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
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
    const { flags } = await this.parse(WithdrawMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const mint = parseAddressFlag(flags.mint, "mint");
    const amount = parseU64StringFlag(flags.amount, "amount");
    if (amount === 0n) {
      this.error("Amount must be greater than zero.");
    }

    const operatorAddress = await findOperator(multisigAuthority.address);
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

    const createCustodianAtaInstruction =
      await getCreateAssociatedTokenIdempotentInstructionAsync({
        mint,
        payer: multisigAuthority,
        owner: custodianAddress,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
      });

    const withdrawInstruction = getWithdrawInstruction({
      operatorAuthority: multisigAuthority,
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

    this.logger.info("Queueing collateral withdrawal:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
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
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) =>
        appendTransactionMessageInstruction(createCustodianAtaInstruction, m),
      (m) => appendTransactionMessageInstruction(withdrawInstruction, m),
    );

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
