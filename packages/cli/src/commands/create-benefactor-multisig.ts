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
import { getCreateBenefactorInstructionAsync } from "jup-stable-sdk";
import { parseBenefactorFeeRateFlag } from "../utils/benefactor";
import { findBenefactor, findOperator } from "jup-stable-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class CreateBenefactor extends BaseCommand {
  static summary = "Create a new benefactor PDA using Squad's Multisig.";

  static description = `
This command creates a new Squad's Multisig Transaction for creating a new benefactor PDA for the supplied authority and initializes it with the provided fee rates.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
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
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateBenefactor);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

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

    const operatorAccount = await findOperator(multisigAuthority.address);
    const benefactorAccount = await findBenefactor(benefactorAuthority);

    this.logger.info("Creating benefactor with:");
    this.logger.info(`  Benefactor authority: ${benefactorAuthority}`);
    this.logger.info(`  Benefactor PDA: ${benefactorAccount}`);
    this.logger.info(`  Mint fee rate: ${mintFeeRate} bps`);
    this.logger.info(`  Redeem fee rate: ${redeemFeeRate} bps`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);

    const instruction = await getCreateBenefactorInstructionAsync({
      operatorAuthority: multisigAuthority,
      operator: operatorAccount,
      payer: multisigAuthority,
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
