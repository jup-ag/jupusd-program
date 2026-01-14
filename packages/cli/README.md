## ⚙️ Configuration Management

### Pause

Pause minting and redeeming operations:

```bash
jup-stable update-config --action=pause
```

---

### Enable Minting and Redeeming

Re-enable mint/redeem operations after a pause:

```bash
jup-stable update-config --action=update-pause-flag --mint-redeem-enabled=true
```

---

### Update Global Period Limit

Set hourly limits on minting and redeeming:

```bash
jup-stable update-config \
 --action=update-period-limit \
 --index=0 \
 --duration-seconds=3600 \
 --max-mint-amount=500000000000 \
 --max-redeem-amount=750000000000
```

**Parameters:**

- `--index`: Period limit index (usually `0` for the first configuration)
- `--duration-seconds`: Duration of each limit window (e.g., `3600` for 1 hour)
- `--max-mint-amount`: Maximum amount that can be minted during the period
- `--max-redeem-amount`: Maximum amount that can be redeemed during the period

---

### Set Peg Price (USD)

Update the asset’s peg price in USD (e.g., for re-pegging or price adjustment):

```bash
jup-stable update-config --action=set-peg-price-usd --peg-price-usd=1.0025
```

**Parameter:**

- `--peg-price-usd`: Peg price to set (in USD)

---

### Create Operator (Admin)

Create a new operator with admin privileges:

```bash
jup-stable create-operator --role=admin --new-operator-authority=<pubkey>
```

---

### Create Benefactor

Create a benefactor:

```bash
jup-stable create-benefactor \
 --benefactor-authority=<pubkey> \
 --mint-fee-rate=25 \
 --redeem-fee-rate=25
```

---

### Create Benefactor with Limits (Multisig)

Create a benefactor with fees, period limits, and enable it using Squad's Multisig:

```bash
jup-stable create-benefactor-with-limits-multisig \
 --multisig=<multisig_pubkey> \
 --benefactor-authority=<pubkey> \
 --mint-fee-rate=25 \
 --redeem-fee-rate=25 \
 --hourly-max-mint=1000000000000 \
 --hourly-max-redeem=1000000000000 \
 --daily-max-mint=24000000000000 \
 --daily-max-redeem=24000000000000
```

**Parameters:**

- `--multisig`: Base58 address of the Squad's multisig program
- `--benefactor-authority`: Base58 address of the authority that controls the benefactor PDA
- `--mint-fee-rate`: Mint fee rate in basis points (0-10000)
- `--redeem-fee-rate`: Redeem fee rate in basis points (0-10000)
- `--hourly-max-mint`: (Optional) Maximum mint amount (raw units) per hour
- `--hourly-max-redeem`: (Optional) Maximum redeem amount (raw units) per hour
- `--daily-max-mint`: (Optional) Maximum daily mint amount (raw units) per day
- `--daily-max-redeem`: (Optional) Maximum redeem amount (raw units) per day

**Note:** This command automatically enables the benefactor after creation. The transaction is queued in the multisig and requires approval from the configured signers.

---

### Update Benefactor

Manage an existing benefactor:

```bash
jup-stable update-benefactor \
 --benefactor-authority=<pubkey> \
 --action=update-fee-rates \
 --mint-fee-rate=10 \
 --redeem-fee-rate=15
```

Actions `disable`, `set-status`, `update-fee-rates`, `update-period-limit`, and `reset-period-limit` are supported.

---

### Update Operator

Update a given operator role:

```bash
jup-stable update-operator --action=set-role --role=peg-manager --managed-operator-authority=<pubkey>
```

---

### Create Vault

Create the vault PDA and token account for a collateral mint (requires a vault manager operator):

```bash
jup-stable create-vault --mint=<mint_pubkey>
```

Optionally pass `--payer-keypair-file=/path/to/keypair.json` to fund from a different signer.

---

### Update Vault

Manage an existing vault:

```bash
jup-stable update-vault --mint=<mint_pubkey> --action=set-status --status=enabled
```

Actions `disable`, `update-oracle`, `update-period-limit`, `reset-period-limit`, `set-custodian`, `set-stalesness-threshold`, `set-min-oracle-price`, and `set-max-oracle-price`.

---
