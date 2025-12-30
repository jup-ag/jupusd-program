import { Flags, Interfaces } from "@oclif/core";
import BaseCommand from "../base-command";
import { parseAddressFlag, parseU64StringFlag } from "../utils/common";
import {
  AccountRole,
  address,
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
import {
  fetchBenefactor,
  fetchConfig,
  fetchVault,
  getMintInstructionAsync,
  JUP_STABLE_PROGRAM_ADDRESS,
} from "jup-stable-sdk";
import { findBenefactor, findConfig, findVault } from "jup-stable-sdk";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";

export default class Mint extends BaseCommand {
  static summary = "Mint Stablecoin against collateral.";

  static description = `
This command mints Stablecoin by depositing collateral into the selected vault.
`;

  static flags = {
    ...BaseCommand.flags,
    "collateral-mint": Flags.string({
      description: "Base58 address of the collateral you want to deposit.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    amount: Flags.string({
      description: "Amount of collateral (u64) to deposit when minting.",
      required: true,
      parse: async (input) => input.trim(),
    }),
    "min-amount-out": Flags.string({
      description:
        "Minimum amount of LP tokens (u64) expected after slippage checks (defaults to 0).",
      required: false,
      parse: async (input) => input.trim(),
    }),
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(Mint);

    this.configureRpcClients();

    const userSigner = this.getSolanaKeypair();
    assertIsTransactionSigner(userSigner);

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
      owner: userSigner.address,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    const [stablecoinTokenAccount] = await findAssociatedTokenPda({
      mint: configAccount.data.mint as Address,
      owner: userSigner.address,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    const amount = parseU64StringFlag(flags.amount, "amount");
    const minAmountOut =
      flags["min-amount-out"] !== undefined
        ? parseU64StringFlag(flags["min-amount-out"], "min-amount-out")
        : 0n;

    const userAddress = userSigner.address as Address;
    const benefactorAddress = await findBenefactor(userAddress);
    const vaultAddress = await findVault(vaultMint);

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

    const [custodianTokenAccount] = await findAssociatedTokenPda({
      mint: vaultMint,
      owner: vaultAccount.data.custodian,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    const authorityAddress = configAccount.data.authority as Address;
    const stablecoinMintAddress = configAccount.data.mint as Address;
    const lpTokenProgramAddress = configAccount.data.tokenProgram as Address;
    const vaultTokenProgramAddress = vaultAccount.data.tokenProgram as Address;

    this.logger.info("Minting Jupiter Stable LP tokens with:");
    this.logger.info(`  User authority: ${userSigner.address}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAddress}`);
    this.logger.info(`  Config PDA: ${configAddress}`);
    this.logger.info(`  Config authority: ${authorityAddress}`);
    this.logger.info(`  Stablecoin mint: ${stablecoinMintAddress}`);
    this.logger.info(`  Vault mint: ${vaultMint}`);
    this.logger.info(`  Vault PDA: ${vaultAddress}`);
    this.logger.info(`  Custodian token account: ${custodianTokenAccount}`);
    this.logger.info(
      `  User collateral token account: ${collateralTokenAccount}`,
    );
    this.logger.info(`  User LP token account: ${stablecoinTokenAccount}`);
    this.logger.info(`  Amount: ${amount.toString()}`);
    this.logger.info(`  Min amount out: ${minAmountOut.toString()}`);

    const createAtaInstruction =
      await getCreateAssociatedTokenIdempotentInstructionAsync({
        mint: stablecoinMintAddress,
        payer: userSigner,
        owner: userSigner.address,
      });

    const instruction = await getMintInstructionAsync({
      user: userSigner,
      userCollateralTokenAccount: collateralTokenAccount,
      userLpTokenAccount: stablecoinTokenAccount,
      config: configAddress,
      authority: authorityAddress,
      lpMint: stablecoinMintAddress,
      vault: vaultAddress,
      custodian: vaultAccount.data.custodian,
      custodianTokenAccount,
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
      (m) => setTransactionMessageFeePayerSigner(userSigner, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(createAtaInstruction, m),
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
      console.error(error);
      this.logger.error(`Mint failed: ${(error as Error).message}`);
      process.exit(1);
    }
  }
}
