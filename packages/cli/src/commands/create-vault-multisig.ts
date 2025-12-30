import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import bs58 from "bs58";

import { parseAddressFlag } from "../utils/common";
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
import { fetchConfig, getCreateVaultInstructionAsync } from "jup-stable-sdk";
import {
  findConfig,
  findOperator,
  findVault,
  findVaultTokenAccount,
} from "jup-stable-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class CreateVaultMultisig extends BaseCommand {
  static summary = "Create a new vault PDA using Squad's Multisig.";

  static description = `
This command creates a new Squad's Multisig transaction that will create the vault PDA and associated token account for the provided collateral mint.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    mint: Flags.string({
      description: "Base58 address of the vault's mint.",
      required: true,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateVaultMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const mint = parseAddressFlag(flags.mint, "mint");

    const operatorAccount = await findOperator(multisigAuthority.address);
    const configAddress = await findConfig();

    let configAccount: Awaited<ReturnType<typeof fetchConfig>> | null = null;
    try {
      configAccount = await fetchConfig(this.rpc, configAddress);
    } catch (error) {
      this.logger.error(
        `Failed to fetch config account at ${configAddress}: ${(error as Error).message}`,
      );
      process.exit(1);
    }

    if (!configAccount) {
      return;
    }

    const authorityAddress = configAccount.data.authority;
    const tokenProgramAddress = configAccount.data.tokenProgram;

    const vaultAddress = await findVault(mint);
    const vaultTokenAccountAddress = await findVaultTokenAccount(mint);

    this.logger.info("Creating vault with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Collateral mint: ${mint}`);
    this.logger.info(`  Config PDA: ${configAddress}`);
    this.logger.info(`  Config authority: ${authorityAddress}`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Vault token account: ${vaultTokenAccountAddress}`);

    const instruction = await getCreateVaultInstructionAsync({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      payer: multisigAuthority,
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
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(instruction, m),
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
