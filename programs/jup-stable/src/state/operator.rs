use std::mem::size_of;

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use static_assertions::const_assert_eq;

use crate::error::JupStableError;

const_assert_eq!(Operator::MAX_SIZE, size_of::<Operator>());

pub const OPERATOR_PREFIX: &[u8; 8] = b"operator";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum OperatorStatus {
    Enabled,
    Disabled,
}

unsafe impl Pod for OperatorStatus {}
unsafe impl Zeroable for OperatorStatus {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum OperatorRole {
    Admin = 0,
    PeriodManager = 1,
    GlobalDisabler = 2,
    VaultManager = 3,
    VaultDisabler = 4,
    BenefactorManager = 5,
    BenefactorDisabler = 6,
    PegManager = 7,
    CollateralManager = 8,
}

#[account(zero_copy)]
pub struct Operator {
    pub operator_authority: Pubkey,
    pub role: u64,
    pub status: OperatorStatus,
    pub _padding0: [u8; 7],
    pub reserved: [u8; 128],
}

impl Default for Operator {
    fn default() -> Self {
        Operator {
            operator_authority: Pubkey::default(),
            role: 0,
            status: OperatorStatus::Disabled,
            _padding0: [0; 7],
            reserved: [0; 128],
        }
    }
}

impl Operator {
    pub const MAX_SIZE: usize = 32 + 8 + 1 + 7 + 128;

    pub fn is(&self, role: OperatorRole) -> Result<()> {
        require!(
            self.status == OperatorStatus::Enabled,
            JupStableError::OperatorDisabled
        );
        require!(
            self.role & (1 << role as u64) != 0,
            JupStableError::InvalidAuthority
        );
        Ok(())
    }

    pub fn set_role(&mut self, role: OperatorRole) { self.role |= 1 << role as u64; }

    pub fn clear_role(&mut self, role: OperatorRole) { self.role &= !(1 << role as u64); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_is() {
        let mut operator = Operator {
            status: OperatorStatus::Enabled,
            ..Operator::default()
        };

        for role_u8 in 0u8..8 {
            let role = match role_u8 {
                0 => OperatorRole::Admin,
                1 => OperatorRole::PeriodManager,
                2 => OperatorRole::GlobalDisabler,
                3 => OperatorRole::VaultManager,
                4 => OperatorRole::VaultDisabler,
                5 => OperatorRole::BenefactorManager,
                6 => OperatorRole::BenefactorDisabler,
                7 => OperatorRole::PegManager,
                _ => continue,
            };
            assert!(
                operator.is(role).is_err(),
                "Operator should not be a {:?}",
                role
            );
        }

        operator.set_role(OperatorRole::PeriodManager);
        assert!(operator.is(OperatorRole::PeriodManager).is_ok());

        operator.clear_role(OperatorRole::PeriodManager);
        assert!(operator.is(OperatorRole::PeriodManager).is_err());
    }
}
