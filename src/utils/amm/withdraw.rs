use crate::error;

/// Calculates withdraw amounts for LP tokens
///
/// # Arguments
///
/// * `lp_token` - Amount of LP tokens to withdraw
/// * `slippage` - Maximum acceptable slippage percentage (0-100)
/// * `base_reserve` - Current reserve of base token in the pool
/// * `quote_reserve` - Current reserve of quote token in the pool
/// * `total_lp_tokens` - Total supply of LP tokens
///
/// # Returns
///
/// Returns a tuple of (base, quote, min_base, min_quote)
pub fn withdraw_lp_token(
    lp_token: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    total_lp_tokens: u64,
) -> Result<(u64, u64, u64, u64), error::ClientError> {
    if lp_token == 0 || total_lp_tokens == 0 {
        return Err(error::ClientError::OtherError(
            "LP token or total LP tokens cannot be zero".into(),
        ));
    }

    if slippage > 100 {
        return Err(error::ClientError::OtherError(
            "Slippage must be between 0 and 100".into(),
        ));
    }

    // Calculate the base and quote amounts
    let base = (base_reserve as u128)
        .checked_mul(lp_token as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(total_lp_tokens as u128)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))? as u64;

    let quote = (quote_reserve as u128)
        .checked_mul(lp_token as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(total_lp_tokens as u128)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))? as u64;

    // Calculate minimum amounts considering slippage
    let scale_factor = 1_000_000_000;
    let slippage_factor = ((100 - slippage as u128) * scale_factor) / 100;

    let min_base = (base as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(scale_factor)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    let min_quote = (quote as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(scale_factor)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    Ok((base, quote, min_base, min_quote))
}
