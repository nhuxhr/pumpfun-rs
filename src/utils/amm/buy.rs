use crate::error;

use super::{fee, MAX_FEE_BASIS_POINTS};

/// Calculates a "buy" in a constant-product AMM with fees
///
/// # Arguments
///
/// * `base` - Base tokens requested (out)
/// * `slippage` - Slippage tolerance in percentage (1 => 1%)
/// * `base_reserve` - Reserve of base token in the pool
/// * `quote_reserve` - Reserve of quote token in the pool
/// * `lp_fee_bps` - LP fee in basis points
/// * `protocol_fee_bps` - Protocol fee in basis points
///
/// # Returns
///
/// Returns a tuple of (internal_quote_amount, ui_quote, max_quote)
pub fn buy_base_input(
    base: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    lp_fee_bps: u64,
    protocol_fee_bps: u64,
) -> Result<(u64, u64, u64), error::ClientError> {
    // Basic validations
    if base == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'base' cannot be zero".into(),
        ));
    }
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero".into(),
        ));
    }
    if base > base_reserve {
        return Err(error::ClientError::OtherError(
            "Cannot buy more base tokens than the pool reserves".into(),
        ));
    }

    // Calculate raw quote needed
    let numerator =
        (quote_reserve as u128)
            .checked_mul(base as u128)
            .ok_or(error::ClientError::OtherError(
                "Multiplication overflow".into(),
            ))?;

    let denominator =
        (base_reserve as u128)
            .checked_sub(base as u128)
            .ok_or(error::ClientError::OtherError(
                "Subtraction underflow".into(),
            ))?;

    if denominator == 0 {
        return Err(error::ClientError::OtherError(
            "Pool would be depleted; denominator is zero".into(),
        ));
    }

    let quote_amount_in = numerator.div_ceil(denominator) as u64;

    // Calculate fees
    let lp_fee = fee(quote_amount_in, lp_fee_bps)?;
    let protocol_fee = fee(quote_amount_in, protocol_fee_bps)?;
    let total_quote = quote_amount_in
        .checked_add(lp_fee)
        .ok_or(error::ClientError::OtherError(
            "Fee addition overflow".into(),
        ))?
        .checked_add(protocol_fee)
        .ok_or(error::ClientError::OtherError(
            "Fee addition overflow".into(),
        ))?;

    // Calculate maxQuote with slippage
    let precision = 1_000_000_000u128;
    let slippage_factor = ((100 + slippage as u128) * precision) / 100;

    let max_quote = (total_quote as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Slippage calculation overflow".into(),
        ))?
        .checked_div(precision)
        .ok_or(error::ClientError::OtherError(
            "Slippage division by zero".into(),
        ))? as u64;

    Ok((quote_amount_in, total_quote, max_quote))
}

/// Calculates a "buy" in a constant-product AMM with fees, where the input is quote tokens
///
/// # Arguments
///
/// * `quote` - Quote tokens provided (in), including fees
/// * `slippage` - Slippage tolerance in percentage (1 => 1%)
/// * `base_reserve` - Reserve of base token in the pool
/// * `quote_reserve` - Reserve of quote token in the pool
/// * `lp_fee_bps` - LP fee in basis points
/// * `protocol_fee_bps` - Protocol fee in basis points
///
/// # Returns
///
/// Returns a tuple of (internal_quote_without_fees, base, max_quote)
pub fn buy_quote_input(
    quote: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    lp_fee_bps: u64,
    protocol_fee_bps: u64,
) -> Result<(u64, u64, u64), error::ClientError> {
    // Basic validations
    if quote == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'quote' cannot be zero".into(),
        ));
    }
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(error::ClientError::OtherError(
            "Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero".into(),
        ));
    }

    // Calculate total fee basis points and denominator
    let total_fee_bps =
        lp_fee_bps
            .checked_add(protocol_fee_bps)
            .ok_or(error::ClientError::OtherError(
                "Fee addition overflow".into(),
            ))?;
    let denominator =
        MAX_FEE_BASIS_POINTS
            .checked_add(total_fee_bps)
            .ok_or(error::ClientError::OtherError(
                "Denominator addition overflow".into(),
            ))?;

    // Calculate effective quote amount
    let effective_quote = (quote as u128)
        .checked_mul(MAX_FEE_BASIS_POINTS as u128)
        .ok_or(error::ClientError::OtherError(
            "Quote calculation overflow".into(),
        ))?
        .checked_div(denominator as u128)
        .ok_or(error::ClientError::OtherError(
            "Quote division by zero".into(),
        ))? as u64;

    // Calculate base tokens received
    let numerator = (base_reserve as u128)
        .checked_mul(effective_quote as u128)
        .ok_or(error::ClientError::OtherError(
            "Base calculation overflow".into(),
        ))?;

    let denominator_effective = (quote_reserve as u128)
        .checked_add(effective_quote as u128)
        .ok_or(error::ClientError::OtherError(
            "Denominator addition overflow".into(),
        ))?;

    if denominator_effective == 0 {
        return Err(error::ClientError::OtherError(
            "Pool would be depleted; denominator is zero".into(),
        ));
    }

    let base_amount_out =
        numerator
            .checked_div(denominator_effective)
            .ok_or(error::ClientError::OtherError(
                "Base division by zero".into(),
            ))? as u64;

    // Calculate maxQuote with slippage
    let precision = 1_000_000_000u128;
    let slippage_factor = ((100 + slippage as u128) * precision) / 100;

    let max_quote = (quote as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Slippage calculation overflow".into(),
        ))?
        .checked_div(precision)
        .ok_or(error::ClientError::OtherError(
            "Slippage division by zero".into(),
        ))? as u64;

    Ok((effective_quote, base_amount_out, max_quote))
}
