import { Flags } from "@oclif/core";
import {
  address,
  appendTransactionMessageInstruction,
  createNoopSigner,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
} from "@solana/kit";
import { PublicKey, VersionedTransaction } from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import bs58 from "bs58";
import { findOperator, getManageOperatorInstruction } from "jupusd-sdk";

import BaseCommand from "../base-command";
import { parseAddressFlag } from "../utils/common";
import { OPERATOR_ROLE_NAMES, ROLE_NAME_TO_ROLE } from "../utils/operator";

export default class GrantAllRolesMultisig extends BaseCommand {
  static summary = "Queue a grant-all-roles action via Squad's Multisig.";

  static description = `
This command generates a multisig transaction that grants ALL available roles to a target operator.
It bundles multiple 'manageOperator' instructions (one for each role) into a single transaction.
The invoking signer must be a multisig member and the multisig authority must already be an enabled Admin operator.
`;

  static flags = {
    ...BaseCommand.flags,
    "managed-operator-authority": Flags.string({
      description:
        "Base58 address of the authority that controls the operator PDA to manage.",
      parse: (input) => Promise.resolve(input.trim()),
      required: true,
    }),
    multisig: Flags.string({
      description: "Base58 address of the multisig program.",
      parse: (input) => Promise.resolve(input.trim()),
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(GrantAllRolesMultisig);

    this.configureRpcClients();

    const multisigPublicKey = new PublicKey(flags["multisig"]);
    // Squads v4 constant for default vault index is 0
    const [vaultPda] = multisig.getVaultPda({
      index: 0,
      multisigPda: multisigPublicKey,
    });

    const multisigAuthority = createNoopSigner(address(vaultPda.toBase58()));

    const managedOperatorAuthority = parseAddressFlag(
      flags["managed-operator-authority"],
      "managed-operator-authority",
    );

    const operatorAccount = await findOperator(multisigAuthority.address);
    const managedOperatorAccount = await findOperator(managedOperatorAuthority);

    this.logger.info("Preparing transaction to grant all roles...");
    this.logger.info(`  Multisig: ${multisigPublicKey.toBase58()}`);
    this.logger.info(
      `  Multisig authority (Vault): ${multisigAuthority.address}`,
    );
    this.logger.info(`  Operator PDA (Admin): ${operatorAccount}`);
    this.logger.info(
      `  Managed operator authority: ${managedOperatorAuthority}`,
    );
    this.logger.info(`  Managed operator PDA: ${managedOperatorAccount}`);

    const instructions = [];

    // Iterate through all roles and create a SetRole instruction for each
    for (const roleName of OPERATOR_ROLE_NAMES) {
      const roleEnum = ROLE_NAME_TO_ROLE[roleName];
      this.logger.info(`  - Adding instruction for role: ${roleName}`);

      const instruction = getManageOperatorInstruction({
        action: {
          __kind: "SetRole",
          role: roleEnum,
        },
        managedOperator: managedOperatorAccount,
        operator: operatorAccount,
        operatorAuthority: multisigAuthority,
      });
      instructions.push(instruction);
    }

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();

    // Start pipe with createTransactionMessage
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let transactionMessage: any = createTransactionMessage({ version: 0 });

    // Set fee payer
    transactionMessage = setTransactionMessageFeePayerSigner(
      multisigAuthority,
      transactionMessage,
    );

    // Set lifetime
    transactionMessage = setTransactionMessageLifetimeUsingBlockhash(
      latestBlockhash,
      transactionMessage,
    );

    // Append all instructions
    for (const ix of instructions) {
      transactionMessage = appendTransactionMessageInstruction(
        ix,
        transactionMessage,
      );
    }

    const base64EncodedWireTransaction = getBase64EncodedWireTransaction(
      await partiallySignTransactionMessageWithSigners(
        transactionMessage as any,
      ),
    );
    const transaction = VersionedTransaction.deserialize(
      Buffer.from(base64EncodedWireTransaction.toString(), "base64"),
    );

    this.logger.info(
      "\nInner transaction (unsigned, base58) - Submit this to your Multisig:",
    );
    this.logger.info(`${bs58.encode(transaction.serialize())}`);
  }
}
