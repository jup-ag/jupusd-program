# JupUSD Monorepo

Monorepo for the **JupUSD** mint & redeem Solana program and tooling:

- **Onchain program (Anchor)**: `jup-stable`
- **TypeScript SDK**: `jupusd-sdk` (generated + quote helpers)
- **CLI**: `jup-stable` (admin/operator workflows)

## Repository layout

- `programs/`
  - `jup-stable/`: core stablecoin program
- `packages/`
  - `sdk/`: `jupusd-sdk` (generated clients + quote utilities)
  - `cli/`: `jup-stable-cli` (ships the `jup-stable` binary)
- `test-utils/`: shared Rust test helpers/fixtures
- `Anchor.toml`: Anchor workspace + localnet program IDs

## Prerequisites

- **Node.js**: `>= 24` (see root `package.json`)
- **pnpm**: `pnpm@10.20.0`
- **Rust**: stable toolchain
- **Solana + Anchor**:
  - `solana` CLI installed and configured
  - Anchor `0.32.1` (see `Anchor.toml`)

## Common workflows

### Build TypeScript packages

```bash
pnpm install && pnpm build
```

### Build program + IDL

From the repo root:

```bash
anchor build
```

This produces artifacts under `target/` including the IDL (e.g. `target/idl/jup_stable.json`).

### Generate the SDK from the IDL

The SDK’s generated clients live in `packages/sdk/src/generated`. Regenerate them after changing the program/IDL:

```bash
anchor build
pnpm run codama
```

(`pnpm run codama` runs Codama using `codama.json`.)

## Packages

### SDK (`jupusd-sdk`)

Located at `packages/sdk`. It bundles:

- Generated account/instruction helpers
- Pure TS quote utilities mirroring program math

See `packages/sdk/README.md` for usage examples and transaction-building patterns.

### CLI (`jup-stable`)

Located at `packages/cli`. The CLI is built with oclif and exposes admin/operator commands (create/update config, operators, benefactors, vaults, etc.).

## License

See `LICENSE`.
