use anchor_lang::prelude::Pubkey;

pub mod add_liquidity;
pub mod add_liquidity_by_strategy;
pub mod add_liquidity_by_strategy_one_side;
pub mod add_liquidity_by_weight;
pub mod add_liquidity_one_side;
pub mod claim_fee;
pub mod claim_reward;
pub mod close_position;
pub mod close_preset_parameter;
pub mod fund_reward;
pub mod increase_oracle_length;
pub mod initialize_bin_array;
pub mod initialize_bin_array_bitmap_extension;
pub mod initialize_lb_pair;
pub mod initialize_permission_lb_pair;
pub mod initialize_position;
pub mod initialize_position_pda;
pub mod initialize_preset_parameters;
pub mod initialize_reward;
pub mod migrate_bin_array;
pub mod migrate_position;
pub mod remove_all_liquidity;
pub mod remove_liquidity;
pub mod swap;
pub mod toggle_pair_status;
pub mod update_fee_owner;
pub mod update_fee_parameters;
pub mod update_fees_and_rewards;
pub mod update_reward_duration;
pub mod update_reward_funder;
pub mod update_whitelisted_wallet;
mod utils;
pub mod withdraw_ineligible_reward;
pub mod withdraw_protocol_fee;
#[cfg(feature = "alpha-access")]
use utils::*;

fn assert_eq_admin(admin: Pubkey) -> bool {
    admin.eq(&crate::admin::ID)
}
