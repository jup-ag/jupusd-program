import {
  address as coerceAddress,
  type Address,
  createKeyPairFromBytes,
  createSignerFromKeyPair,
} from "@solana/kit";
import { readFile } from "fs/promises";

export async function parseKeypairFile(path: string) {
  const raw = await readFile(path, "utf8");
  const parsed = JSON.parse(raw);

  if (!Array.isArray(parsed)) {
    throw new Error(
      `Expected keypair file at ${path} to contain an array of bytes.`,
    );
  }

  return createSignerFromKeyPair(
    await createKeyPairFromBytes(new Uint8Array(parsed)),
  );
}

export function parseBooleanFlag(raw: string, flagName: string): boolean {
  const normalized = raw.trim().toLowerCase();
  if (
    ["true", "t", "1", "yes", "y", "on", "enable", "enabled"].includes(
      normalized,
    )
  ) {
    return true;
  }

  if (
    ["false", "f", "0", "no", "n", "off", "disable", "disabled"].includes(
      normalized,
    )
  ) {
    return false;
  }

  throw new Error(
    `Invalid value for --${flagName}. Expected a boolean-like value (true/false). Received: ${raw}`,
  );
}

export function parseU64StringFlag(raw: string, flagName: string): bigint {
  if (!/^\d+$/.test(raw)) {
    throw new Error(
      `Invalid value for --${flagName}. Expected a non-negative integer but received: ${raw}`,
    );
  }

  try {
    const value = BigInt(raw);
    const max = (1n << 64n) - 1n;
    if (value > max) {
      throw new Error(
        `Value for --${flagName} exceeds the maximum u64 (${max.toString()}). Received: ${raw}`,
      );
    }

    return value;
  } catch (error) {
    throw new Error(
      `Unable to parse --${flagName} as a 64-bit unsigned integer: ${(error as Error).message}`,
    );
  }
}

export function parseAddressFlag(value: string, flagName: string): Address {
  try {
    return coerceAddress(value);
  } catch (error) {
    throw new Error(
      `Invalid ${flagName} value. Expected a base58 Solana address. ${(error as Error).message}`,
    );
  }
}
