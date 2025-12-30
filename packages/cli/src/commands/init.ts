import { Flags } from "@oclif/core";
import { stdin as input, stdout as output } from "node:process";
import { createInterface } from "node:readline/promises";
import BaseCommand from "../base-command";
import { parseKeypairFile } from "../utils/common";
import {
  getAddressEncoder,
  fetchJsonParsedAccount,
  JsonParsedBpfUpgradeableLoaderProgramAccount,
  Address,
  getProgramDerivedAddress,
  getBytesEncoder,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstruction,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  assertIsSendableTransaction,
  assertIsTransactionWithBlockhashLifetime,
  assertIsTransactionSigner,
  getSignatureFromTransaction,
} from "@solana/kit";
import {
  getInitInstructionAsync,
  JUP_STABLE_PROGRAM_ADDRESS,
} from "jup-stable-sdk";

export default class Init extends BaseCommand {
  static summary = "Initialize The Jupiter Stable Program.";

  static description = `
This command will call the init instruction to initialize the Jupiter Stable program.
`;

  static flags = {
    ...BaseCommand.flags,
    decimals: Flags.integer({
      description: "Token decimal precision to use when initializing.",
      required: true,
      min: 0,
      max: 18,
    }),
    name: Flags.string({
      description: "Token display name.",
      required: true,
    }),
    symbol: Flags.string({
      description: "Token ticker symbol.",
      required: true,
    }),
    uri: Flags.string({
      description: "Metadata URI associated with the token.",
      required: true,
    }),
    "mint-keypair-file": Flags.string({
      description: "Path to the mint authority keypair file.",
      required: true,
      parse: async (input, _ctx) => {
        if (!input.trim()) {
          throw new Error("mint_keypair_file cannot be empty.");
        }

        return input;
      },
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Init);

    this.configureRpcClients();

    const signerKeypair = this.getSolanaKeypair();
    const mintKeypair = await parseKeypairFile(flags["mint-keypair-file"]);

    this.logger.info("Initializing program with:");
    this.logger.info(`  Payer: ${signerKeypair.address}`);
    this.logger.info(`  Name: ${flags.name}`);
    this.logger.info(`  Symbol: ${flags.symbol}`);
    this.logger.info(`  Decimals: ${flags.decimals}`);
    this.logger.info(`  URI: ${flags.uri}`);
    this.logger.info(`  Mint: ${mintKeypair.address}`);

    type BpfUpgradeableLoaderAccountInfo =
      JsonParsedBpfUpgradeableLoaderProgramAccount["info"];
    const programAccount =
      await fetchJsonParsedAccount<BpfUpgradeableLoaderAccountInfo>(
        this.rpc,
        JUP_STABLE_PROGRAM_ADDRESS,
      );

    if (!programAccount.exists) {
      this.logger.error("Program account does not exist");
      process.exit(1);
    }

    const programAccountData = programAccount.data;

    if (!("programData" in programAccountData)) {
      this.logger.error(
        "Program account is not a BPFLoaderUpgradeable account",
      );
      process.exit(1);
    }

    const confirmationInterface = createInterface({ input, output });
    let answer = "";

    try {
      await this.logger.flush();
      answer = await confirmationInterface.question(
        "Continue initializing the Jupiter Stable program? (y/n) ",
      );
    } finally {
      confirmationInterface.close();
    }

    if (!/^(y|yes)$/i.test(answer.trim())) {
      this.logger.info("Initialization cancelled.");
      return;
    }

    const programDataAddress = programAccountData.programData;
    const mint = mintKeypair.address as Address;
    const metadataProgram =
      "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" as Address;
    const [metadata, _bumpSeed] = await getProgramDerivedAddress({
      programAddress: metadataProgram,
      seeds: [
        getBytesEncoder().encode(Buffer.from("metadata")),
        getAddressEncoder().encode(metadataProgram),
        getAddressEncoder().encode(mint),
      ],
    });

    assertIsTransactionSigner(signerKeypair);
    assertIsTransactionSigner(mintKeypair);

    const initInstruction = await getInitInstructionAsync({
      payer: signerKeypair,
      upgradeAuthority: signerKeypair,
      mint: mintKeypair,
      metadata: metadata,
      programData: programDataAddress,
      decimals: flags.decimals,
      name: flags.name,
      symbol: flags.symbol,
      uri: flags.uri,
    });

    const { value: latestBlockhash } = await this.rpc
      .getLatestBlockhash()
      .send();
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(signerKeypair, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstruction(initInstruction, m),
    );

    const transaction =
      await signTransactionMessageWithSigners(transactionMessage);

    assertIsSendableTransaction(transaction);
    assertIsTransactionWithBlockhashLifetime(transaction);

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
    } catch (e) {
      this.logger.error(`Transfer failed: ${e}`);
      process.exit(1);
    }
  }
}
