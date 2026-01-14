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
import { findOperator, getDeleteOperatorInstruction } from "jupusd-sdk";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class DeleteOperatorMultisig extends BaseCommand {
  static summary = "Delete an existing operator PDA using Squad's Multisig.";

  static description = `
This command queues a Squad's Multisig transaction that closes an existing operator PDA and returns its rent to the payer signer.
The invoking signer must be an enabled operator with the Admin role.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
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
    const { flags } = await this.parse(DeleteOperatorMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const operatorAuthority = parseAddressFlag(
      flags["operator-authority"],
      "operator-authority",
    );
    const receiverAddress =
      flags.receiver !== undefined
        ? parseAddressFlag(flags.receiver, "receiver")
        : operatorAuthority;

    const operatorAccount = await findOperator(multisigAuthority.address);
    const deletedOperatorAccount = await findOperator(operatorAuthority);

    this.logger.info("Deleting operator with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Operator PDA: ${operatorAccount}`);
    this.logger.info(`  Deleted operator authority: ${operatorAuthority}`);
    this.logger.info(`  Deleted operator PDA: ${deletedOperatorAccount}`);
    this.logger.info(`  Rent receiver: ${receiverAddress}`);

    const instruction = getDeleteOperatorInstruction({
      operatorAuthority: multisigAuthority,
      payer: multisigAuthority,
      operator: operatorAccount,
      deletedOperator: deletedOperatorAccount,
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
