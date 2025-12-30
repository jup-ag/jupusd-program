use anchor_lang::{prelude::*, solana_program::sysvar::last_restart_slot::LastRestartSlot};
use doves::AgPriceFeed;
use pyth_solana_receiver_sdk::price_update::{Price as PriceV2, PriceUpdateV2};
use rust_decimal::Decimal;
use switchboard_on_demand::PullFeedAccountData;

use crate::{error::JupStableError, state::vault::OracleType};

pub const PYTH_RECEIVER_PROGRAM_ID: Pubkey = pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
pub const SWITCHBOARD_ON_DEMAND_PROGRAM_ID: Pubkey =
    pubkey!("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");

const MAX_CONFIDENCE_BPS: u64 = 200u64;

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct OraclePrice(pub Decimal);

impl OraclePrice {
    fn from_pyth_v2(
        feed_id: &[u8; 32],
        oracle: &AccountInfo,
        clock: &Clock,
        stalesness_threshold: u64,
    ) -> Result<Self> {
        // No longer possible: https://github.com/coral-xyz/anchor/pull/2770
        // let price_feed = Account::<'_, PriceUpdateV2>::try_from(&info).unwrap();
        let price_feed = PriceUpdateV2::try_deserialize(&mut &oracle.data.borrow()[..])?;

        let price: PriceV2 =
            price_feed.get_price_no_older_than(clock, stalesness_threshold, feed_id)?;

        if price.price <= 0 {
            return err!(JupStableError::BadOracle);
        }
        let price_u64: u64 = price.price.try_into()?;

        let scaled_conf = price.conf * 10_000 / MAX_CONFIDENCE_BPS;
        if scaled_conf >= price_u64 {
            return err!(JupStableError::PriceConfidenceTooWide);
        };

        Ok(OraclePrice(Decimal::from_i128_with_scale(
            price_u64.into(),
            price.exponent.abs().try_into()?,
        )))
    }

    fn from_switchboard_on_demand(
        oracle: &AccountInfo,
        clock: &Clock,
        stalesness_threshold: u64,
    ) -> Result<Self> {
        let slot_treshold = stalesness_threshold * 1000 / clock::DEFAULT_MS_PER_SLOT;
        let last_restart_slot = LastRestartSlot::get()?;

        let feed_account = oracle.try_borrow_data()?;
        let price_feed = PullFeedAccountData::parse(feed_account)
            .map_err(|_| error!(JupStableError::BadOracle))?;

        require!(
            price_feed.last_update_slot() > last_restart_slot.last_restart_slot,
            JupStableError::BadOracle
        );

        let price = price_feed
            .get_value(clock.slot, slot_treshold, 1, true)
            .map_err(|_| error!(JupStableError::BadOracle))?;

        require!(
            price_feed.last_update_timestamp + i64::try_from(stalesness_threshold)?
                >= clock.unix_timestamp,
            JupStableError::BadOracle
        );

        let stdev = price_feed
            .result
            .std_dev()
            .ok_or(error!(JupStableError::BadOracle))?;

        if price == Decimal::ZERO {
            return err!(JupStableError::BadOracle);
        }

        let stdev_conf = stdev * Decimal::from(10_000) / Decimal::from(MAX_CONFIDENCE_BPS);
        if stdev_conf >= price {
            return err!(JupStableError::PriceConfidenceTooWide);
        }

        Ok(OraclePrice(price))
    }

    fn from_doves(oracle: &AccountInfo, clock: &Clock, stalesness_threshold: u64) -> Result<Self> {
        let price = AgPriceFeed::try_deserialize(&mut &oracle.data.borrow()[..])?;

        let s: i64 = stalesness_threshold.try_into()?;
        require!(
            price.timestamp + s > clock.unix_timestamp,
            JupStableError::BadOracle
        );

        if price.price == 0 {
            return err!(JupStableError::BadOracle);
        }

        Ok(OraclePrice(Decimal::from_i128_with_scale(
            price.price as i128,
            price.expo.abs().try_into()?,
        )))
    }

    pub fn parse_oracles(
        oracles: &[OracleType],
        oracle_account: &[AccountInfo],
        clock: &Clock,
        stalesness_threshold: u64,
    ) -> Result<Self> {
        let non_empty_oracles: Vec<&OracleType> = oracles
            .iter()
            .filter(|o| !matches!(o, OracleType::Empty(_)))
            .collect();

        if non_empty_oracles.is_empty() {
            return err!(JupStableError::NoOraclesFound);
        }

        require!(
            oracle_account.len() >= non_empty_oracles.len(),
            JupStableError::MissingOracleAccounts,
        );

        let prices: Result<Vec<OraclePrice>> = non_empty_oracles
            .iter()
            .zip(oracle_account.iter())
            .map(
                |(oracle, account_info)| match (oracle, account_info.owner) {
                    (OracleType::Pyth(pyth), &PYTH_RECEIVER_PROGRAM_ID) => {
                        require!(pyth.account == *account_info.key, JupStableError::BadOracle);
                        OraclePrice::from_pyth_v2(
                            &pyth.feed_id,
                            account_info,
                            clock,
                            stalesness_threshold,
                        )
                    },
                    (
                        OracleType::SwitchboardOnDemand(switchboard),
                        &SWITCHBOARD_ON_DEMAND_PROGRAM_ID,
                    ) => {
                        require!(
                            switchboard.account == *account_info.key,
                            JupStableError::BadOracle
                        );
                        OraclePrice::from_switchboard_on_demand(
                            account_info,
                            clock,
                            stalesness_threshold,
                        )
                    },
                    (OracleType::Doves(doves), &doves::ID_CONST) => {
                        require!(
                            doves.account == *account_info.key,
                            JupStableError::BadOracle
                        );
                        OraclePrice::from_doves(account_info, clock, stalesness_threshold)
                    },
                    _ => err!(JupStableError::BadOracle),
                },
            )
            .collect();

        let prices: Vec<OraclePrice> = prices?;

        if prices.len() > 1 {
            let min_price = prices
                .iter()
                .map(|p| p.0)
                .min()
                .ok_or_else(|| error!(JupStableError::NoValidPrice))?;
            let max_price = prices
                .iter()
                .map(|p| p.0)
                .max()
                .ok_or_else(|| error!(JupStableError::NoValidPrice))?;

            // Require that oracle spread stays within confidence bounds.
            let spread_bps = (max_price - min_price) * Decimal::from(10_000u64) / min_price;
            require!(
                spread_bps <= Decimal::from(MAX_CONFIDENCE_BPS),
                JupStableError::PriceConfidenceTooWide
            );
        }

        // Return the most conservative price for collateral
        prices
            .into_iter()
            .min()
            .ok_or_else(|| error!(JupStableError::NoValidPrice))
    }
}
