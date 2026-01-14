import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import bs58 from "bs58";

import { parseAddressFlag, parseU64StringFlag } from "../utils/common";
import {
  AccountRole,
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
  fetchBenefactor,
  getRedeemInstructionAsync,
  JUP_STABLE_PROGRAM_ADDRESS,
} from "jupusd-sdk";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import {
  findBenefactor,
  findConfig,
  findVault,
  findVaultTokenAccount,
} from "jupusd-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class RedeemMultisig extends BaseCommand {
  static summary = "Queue a redemption of Stablecoin via Squad's Multisig.";

  static description = `
This command creates a Squad's Multisig transaction that burns Stablecoin from the multisig vault and withdraws the underlying collateral.
  `;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "collateral-mint": Flags.string({
      description: "Base58 address of the collateral you want to withdraw.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    amount: Flags.string({
      description: "Amount of Stablecoin (u64) to redeem.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "min-amount-out": Flags.string({
      description:
        "Minimum amount of collateral (u64) expected after slippage checks (defaults to 0).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(RedeemMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

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

    const vaultMint = parseAddressFlag(
      flags["collateral-mint"],
      "collateral-mint",
    );

    const [collateralTokenAccount] = await findAssociatedTokenPda({
      mint: vaultMint,
      owner: multisigAuthority.address,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    const [stablecoinTokenAccount] = await findAssociatedTokenPda({
      mint: configAccount.data.mint as Address,
      owner: multisigAuthority.address,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    const amount = parseU64StringFlag(flags.amount, "amount");
    if (amount === 0n) {
      this.error("Amount must be greater than zero.");
    }
    const minAmountOut =
      flags["min-amount-out"] !== undefined
        ? parseU64StringFlag(flags["min-amount-out"], "min-amount-out")
        : 0n;

    const benefactorAddress = await findBenefactor(multisigAuthority.address);
    const vaultAddress = await findVault(vaultMint);
    const vaultTokenAccountAddress = await findVaultTokenAccount(vaultMint);

    try {
      await fetchBenefactor(this.rpc, benefactorAddress as Address);
    } catch (error) {
      this.logger.error(
        `Failed to fetch benefactor account at ${benefactorAddress}: ${(error as Error).message}`,
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
    const stablecoinMintAddress = configAccount.data.mint as Address;
    const lpTokenProgramAddress = configAccount.data.tokenProgram as Address;
    const vaultTokenProgramAddress = vaultAccount.data.tokenProgram as Address;

    this.logger.info("Queueing Stablecoin redemption:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAddress}`);
    this.logger.info(`  Config PDA: ${configAddress}`);
    this.logger.info(`  Config authority: ${authorityAddress}`);
    this.logger.info(`  Stablecoin mint: ${stablecoinMintAddress}`);
    this.logger.info(`  Vault mint: ${vaultMint}`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Vault token account: ${vaultTokenAccountAddress}`);
    this.logger.info(
      `  User collateral token account: ${collateralTokenAccount}`,
    );
    this.logger.info(`  User LP token account: ${stablecoinTokenAccount}`);
    this.logger.info(`  Amount: ${amount.toString()}`);
    this.logger.info(`  Min amount out: ${minAmountOut.toString()}`);

    const createCollateralAtaInstruction =
      await getCreateAssociatedTokenIdempotentInstructionAsync({
        mint: vaultMint,
        payer: multisigAuthority,
        owner: multisigAuthority.address,
      });

    const instruction = await getRedeemInstructionAsync({
      user: multisigAuthority,
      userLpTokenAccount: stablecoinTokenAccount,
      userCollateralTokenAccount: collateralTokenAccount,
      config: configAddress,
      authority: authorityAddress,
      lpMint: stablecoinMintAddress,
      vault: vaultAddress,
      vaultTokenAccount: vaultTokenAccountAddress,
      vaultMint,
      benefactor: benefactorAddress,
      lpTokenProgram: lpTokenProgramAddress,
      vaultTokenProgram: vaultTokenProgramAddress,
      program: JUP_STABLE_PROGRAM_ADDRESS,
      amount,
      minAmountOut,
    });

    instruction.accounts.push({
      role: AccountRole.READONLY,
      address: address("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX"),
    });

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(multisigAuthority, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) =>
        appendTransactionMessageInstruction(createCollateralAtaInstruction, m),
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
