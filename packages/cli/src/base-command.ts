import { Command, Flags, Interfaces } from "@oclif/core";
import {
  createKeyPairFromBytes,
  createSignerFromKeyPair,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  KeyPairSigner,
  mainnet,
} from "@solana/kit";
import { readFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { getLogger, type Logger } from "./utils/logger";

const DEFAULT_SOLANA_KEYPAIR_PATH = join(
  homedir(),
  ".config",
  "solana",
  "id.json",
);

const DEFAULT_RPC_ENDPOINT = "https://api.mainnet-beta.solana.com";

export abstract class BaseCommand extends Command {
  public static readonly flags = {
    rpc: Flags.string({
      description: "Solana RPC endpoint URL used for all CLI commands.",
      env: "RPC_ENDPOINT",
      default: DEFAULT_RPC_ENDPOINT,
    }),
  } satisfies Interfaces.FlagInput;

  protected parsedBaseFlags: Interfaces.InferredFlags<
    typeof BaseCommand.flags
  > | null = null;
  protected solanaKeypair: KeyPairSigner<string> | null = null;
  protected rawSolanaKeypair: Uint8Array | null = null;
  protected solanaKeypairPath = DEFAULT_SOLANA_KEYPAIR_PATH;
  protected rpc = createSolanaRpc(mainnet(DEFAULT_RPC_ENDPOINT));
  protected rpcUrl = DEFAULT_RPC_ENDPOINT;
  protected rpcSubscriptions = createSolanaRpcSubscriptions(
    DEFAULT_RPC_ENDPOINT.replace("https://", "wss://"),
  );
  protected logger: Logger = getLogger();

  async init(): Promise<void> {
    await super.init();
    this.logger = getLogger();

    const ctor = this.constructor as typeof BaseCommand;
    const { flags } = await this.parse(ctor as unknown as typeof BaseCommand);
    this.parsedBaseFlags = flags as Interfaces.InferredFlags<
      typeof BaseCommand.flags
    >;
    this.configureRpcClients();
    await this.loadDefaultSolanaKeypair();
  }

  protected configureRpcClients(): void {
    const rpcFlag = this.parsedBaseFlags?.rpc;

    const rpcEndpoint = rpcFlag?.trim() || DEFAULT_RPC_ENDPOINT;
    this.rpcUrl = rpcEndpoint;
    this.rpc = createSolanaRpc(mainnet(rpcEndpoint));
    this.rpcSubscriptions = createSolanaRpcSubscriptions(
      rpcEndpoint.replace("https://", "wss://"),
    );
  }

  protected async loadDefaultSolanaKeypair(): Promise<void> {
    try {
      const raw = await readFile(this.solanaKeypairPath, "utf8");
      const parsed = JSON.parse(raw);

      if (!Array.isArray(parsed)) {
        this.logger.warn(
          `Expected keypair file at ${this.solanaKeypairPath} to contain an array of bytes.`,
        );
        return;
      }

      this.rawSolanaKeypair = new Uint8Array(parsed);
      this.solanaKeypair = await createSignerFromKeyPair(
        await createKeyPairFromBytes(this.rawSolanaKeypair),
      );
      this.logger.info(
        `Loaded default Solana keypair from ${this.solanaKeypairPath}.`,
      );
    } catch (error) {
      if ((error as NodeJS.ErrnoException)?.code === "ENOENT") {
        this.logger.debug(
          `No default Solana keypair found at ${this.solanaKeypairPath}.`,
        );
        return;
      }

      this.logger.warn(
        `Unable to load Solana keypair from ${this.solanaKeypairPath}: ${(error as Error).message}`,
      );
    }
  }

  protected getSolanaKeypair(): KeyPairSigner<string> {
    if (!this.solanaKeypair) {
      this.error(
        `No default Solana keypair found at ${this.solanaKeypairPath}.`,
      );
    }

    return this.solanaKeypair;
  }

  protected getRawSolanaKeypair(): Uint8Array {
    if (!this.rawSolanaKeypair) {
      this.error(
        `No default Solana keypair found at ${this.solanaKeypairPath}.`,
      );
    }

    return this.rawSolanaKeypair;
  }
}

export default BaseCommand;
