import { JUP_STABLE_PROGRAM_ADDRESS } from "./generated";
import {
  Address,
  getAddressEncoder,
  getBytesEncoder,
  getProgramDerivedAddress,
} from "@solana/kit";
import {
  findAssociatedTokenPda,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";

export async function findConfig(): Promise<Address> {
  const [configAccount] = await getProgramDerivedAddress({
    programAddress: JUP_STABLE_PROGRAM_ADDRESS,
    seeds: [getBytesEncoder().encode(Buffer.from("config"))],
  });
  return configAccount;
}

export async function findAuthority(): Promise<Address> {
  const [authorityAccount] = await getProgramDerivedAddress({
    programAddress: JUP_STABLE_PROGRAM_ADDRESS,
    seeds: [getBytesEncoder().encode(Buffer.from("authority"))],
  });
  return authorityAccount;
}

export async function findOperator(authority: Address): Promise<Address> {
  const [operatorAccount] = await getProgramDerivedAddress({
    programAddress: JUP_STABLE_PROGRAM_ADDRESS,
    seeds: [
      getBytesEncoder().encode(Buffer.from("operator")),
      getAddressEncoder().encode(authority),
    ],
  });
  return operatorAccount;
}

export async function findBenefactor(authority: Address): Promise<Address> {
  const [benefactorAccount] = await getProgramDerivedAddress({
    programAddress: JUP_STABLE_PROGRAM_ADDRESS,
    seeds: [
      getBytesEncoder().encode(Buffer.from("benefactor")),
      getAddressEncoder().encode(authority),
    ],
  });
  return benefactorAccount;
}

export async function findVault(mint: Address): Promise<Address> {
  const [vaultAccount] = await getProgramDerivedAddress({
    programAddress: JUP_STABLE_PROGRAM_ADDRESS,
    seeds: [
      getBytesEncoder().encode(Buffer.from("vault")),
      getAddressEncoder().encode(mint),
    ],
  });
  return vaultAccount;
}

export async function findVaultTokenAccount(mint: Address): Promise<Address> {
  const [vaultTokenAccount] = await findAssociatedTokenPda({
    mint: mint,
    owner: await findAuthority(),
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  return vaultTokenAccount;
}
