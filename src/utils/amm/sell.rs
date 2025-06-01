#![allow(clippy::too_many_arguments)]

use solana_sdk::pubkey::Pubkey;

use crate::error;

use super::{fee, MAX_FEE_BASIS_POINTS};

/// Calculates output for selling base tokens
///
/// # Arguments
///
/// * `base` - The amount of base tokens to sell
/// * `slippage` - Slippage tolerance in percentage (1 => 1%)
/// * `base_reserve` - Current reserve of base tokens in the pool
/// * `quote_reserve` - Current reserve of quote tokens in the pool
/// * `lp_fee_bps` - LP fee in basis points
/// * `protocol_fee_bps` - Protocol fee in basis points
///
/// # Returns
///
/// Returns a tuple of (ui_quote, internal_quote_amount_out, min_quote)
pub fn sell_base_input(
    base: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    lp_fee_bps: u64,
    protocol_fee_bps: u64,
    coin_creator_fee_bps: u64,
    coin_creator: &Pubkey,
) -> Result<(u64, u64, u64), error::ClientError> {
    // Basic validations
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero".into(),
        ));
    }

    // Calculate raw quote output
    let quote_amount_out = (quote_reserve as u128)
        .checked_mul(base as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(
            (base_reserve as u128)
                .checked_add(base as u128)
                .ok_or(error::ClientError::OtherError("Addition overflow".into()))?,
        )
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    // Calculate fees
    let lp_fee = fee(quote_amount_out, lp_fee_bps)?;
    let protocol_fee = fee(quote_amount_out, protocol_fee_bps)?;
    let coin_creator_fee = if Pubkey::default().eq(coin_creator) {
        0
    } else {
        fee(quote_amount_out, coin_creator_fee_bps)?
    };

    let final_quote = quote_amount_out
        .checked_sub(lp_fee)
        .ok_or(error::ClientError::OtherError(
            "Fee subtraction underflow".into(),
        ))?
        .checked_sub(protocol_fee)
        .ok_or(error::ClientError::OtherError(
            "Fee subtraction underflow".into(),
        ))?
        .checked_sub(coin_creator_fee)
        .ok_or(error::ClientError::OtherError(
            "Fee subtraction underflow".into(),
        ))?;

    // Calculate minQuote with slippage
    let precision = 1_000_000_000u128;
    let slippage_factor = ((100 - slippage as u128) * precision) / 100;

    let min_quote = (final_quote as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Slippage calculation overflow".into(),
        ))?
        .checked_div(precision)
        .ok_or(error::ClientError::OtherError(
            "Slippage division by zero".into(),
        ))? as u64;

    Ok((final_quote, quote_amount_out, min_quote))
}

/// Helper function to calculate quote amount out including fees
fn calculate_quote_amount_out(
    user_quote_amount_out: u64,
    lp_fee_bps: u64,
    protocol_fee_bps: u64,
    coin_creator_fee_bps: u64,
) -> Result<u64, error::ClientError> {
    let total_fee_bps = lp_fee_bps
        .checked_add(protocol_fee_bps)
        .ok_or(error::ClientError::OtherError(
            "Fee addition overflow".into(),
        ))?
        .checked_add(coin_creator_fee_bps)
        .ok_or(error::ClientError::OtherError(
            "Fee addition overflow".into(),
        ))?;

    let denominator =
        MAX_FEE_BASIS_POINTS
            .checked_sub(total_fee_bps)
            .ok_or(error::ClientError::OtherError(
                "Fee subtraction underflow".into(),
            ))?;

    let numerator = (user_quote_amount_out as u128)
        .checked_mul(MAX_FEE_BASIS_POINTS as u128)
        .ok_or(error::ClientError::OtherError(
            "Quote calculation overflow".into(),
        ))?;

    Ok(numerator.div_ceil(denominator as u128) as u64)
}

/// Calculates input needed for selling to get desired quote amount
///
/// # Arguments
///
/// * `quote` - Desired quote tokens (including fees)
/// * `slippage` - Slippage tolerance in percentage (1 => 1%)
/// * `base_reserve` - Current reserve of base tokens in the pool
/// * `quote_reserve` - Current reserve of quote tokens in the pool
/// * `lp_fee_bps` - LP fee in basis points
/// * `protocol_fee_bps` - Protocol fee in basis points
///
/// # Returns
///
/// Returns a tuple of (internal_raw_quote, base, min_quote)
pub fn sell_quote_input(
    quote: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    lp_fee_bps: u64,
    protocol_fee_bps: u64,
    coin_creator_fee_bps: u64,
    coin_creator: &Pubkey,
) -> Result<(u64, u64, u64), error::ClientError> {
    // Basic validations
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero".into(),
        ));
    }
    if quote > quote_reserve {
        return Err(error::ClientError::OtherError(
            "Cannot receive more quote tokens than the pool quote reserves".into(),
        ));
    }

    // Calculate raw quote including fees
    let coin_creator_fee_bps = if Pubkey::default().eq(coin_creator) {
        0
    } else {
        coin_creator_fee_bps
    };
    let raw_quote =
        calculate_quote_amount_out(quote, lp_fee_bps, protocol_fee_bps, coin_creator_fee_bps)?;

    if raw_quote >= quote_reserve {
        return Err(error::ClientError::OtherError(
            "Invalid input: Desired quote amount exceeds available reserve".into(),
        ));
    }

    // Calculate base amount needed
    let base_amount_in = {
        let numerator = (base_reserve as u128)
            .checked_mul(raw_quote as u128)
            .ok_or(error::ClientError::OtherError(
                "Base calculation overflow".into(),
            ))?;

        let denominator = (quote_reserve as u128)
            .checked_sub(raw_quote as u128)
            .ok_or(error::ClientError::OtherError(
                "Quote subtraction underflow".into(),
            ))?;

        numerator.div_ceil(denominator) as u64
    };

    // Calculate minQuote with slippage
    let precision = 1_000_000_000u128;
    let slippage_factor = ((100 - slippage as u128) * precision) / 100;

    let min_quote = (quote as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Slippage calculation overflow".into(),
        ))?
        .checked_div(precision)
        .ok_or(error::ClientError::OtherError(
            "Slippage division by zero".into(),
        ))? as u64;

    Ok((raw_quote, base_amount_in, min_quote))
}
