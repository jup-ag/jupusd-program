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
import { getCreateOperatorInstructionAsync } from "jupusd-sdk";
import { findOperator } from "jupusd-sdk";
import { OPERATOR_ROLE_NAMES, parseOperatorRoleFlag } from "../utils/operator";
import * as multisig from "@sqds/multisig";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";

export default class CreateOperatorMultisig extends BaseCommand {
  static summary = "Create a new operator PDA using Squad's Multisig.";

  static description = `
This command creates a new Squad's Multisig transaction that creates an operator PDA for the supplied authority and assigns it the requested role.
`;

  static flags = {
    ...BaseCommand.flags,
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      required: true,
      parse: async (input) => input.trim(),
    }),
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
  } satisfies Interfaces.FlagInput;

  async run(): Promise<void> {
    const { flags } = await this.parse(CreateOperatorMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    const [vaultPda] = multisig.getVaultPda({
      multisigPda: multisigPublicKey,
      index: 0,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const role = parseOperatorRoleFlag(flags.role);
    const newOperatorAuthority = parseAddressFlag(
      flags["new-operator-authority"],
      "new-operator-authority",
    );

    const operatorAccount = await findOperator(multisigAuthority.address);
    const newOperatorAccount = await findOperator(newOperatorAuthority);

    this.logger.info("Creating operator with:");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(`  Multisig authority: ${multisigAuthority.address}`);
    this.logger.info(`  Existing operator PDA: ${operatorAccount}`);
    this.logger.info(`  New operator authority: ${newOperatorAuthority}`);
    this.logger.info(`  New operator PDA: ${newOperatorAccount}`);
    this.logger.info(`  Role: ${role.name}`);

    const instruction = await getCreateOperatorInstructionAsync({
      operatorAuthority: multisigAuthority,
      payer: multisigAuthority,
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
