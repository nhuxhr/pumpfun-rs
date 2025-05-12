use crate::error;

/// Calculates deposit amounts for token0
///
/// # Arguments
///
/// * `token0` - Amount of token0 to deposit
/// * `slippage` - Maximum acceptable slippage percentage (0-100)
/// * `token0_reserve` - Current reserve of token0 in the pool
/// * `token1_reserve` - Current reserve of token1 in the pool
/// * `total_lp_tokens` - Total supply of LP tokens
///
/// # Returns
///
/// Returns a tuple of (token1_amount, lp_tokens, max_token0, max_token1)
pub fn deposit_token0(
    token0: u64,
    slippage: u8,
    token0_reserve: u64,
    token1_reserve: u64,
    total_lp_tokens: u64,
) -> Result<(u64, u64, u64, u64), error::ClientError> {
    if slippage > 100 {
        return Err(error::ClientError::OtherError(
            "Slippage must be between 0 and 100".into(),
        ));
    }

    // Calculate corresponding output amount based on pool reserves
    let token1 = (token0 as u128)
        .checked_mul(token1_reserve as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(token0_reserve as u128)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))? as u64;

    // Apply slippage tolerance
    let slippage_factor = ((1_u128 + (slippage as u128)) * 1_000_000_000) / 100;
    let slippage_denominator = 1_000_000_000;

    let max_token0 = (token0 as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(slippage_denominator)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    let max_token1 = (token1 as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(slippage_denominator)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    // Calculate LP tokens to mint, proportional to deposit amount
    let lp_token = (token0 as u128)
        .checked_mul(total_lp_tokens as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(token0_reserve as u128)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    Ok((token1, lp_token, max_token0, max_token1))
}

/// Calculates deposit amounts for LP tokens
///
/// # Arguments
///
/// * `lp_token` - Amount of LP tokens desired
/// * `slippage` - Maximum acceptable slippage percentage (0-100)
/// * `base_reserve` - Current reserve of base token in the pool
/// * `quote_reserve` - Current reserve of quote token in the pool
/// * `total_lp_tokens` - Total supply of LP tokens
///
/// # Returns
///
/// Returns a tuple of maximum base and quote token amounts needed
pub fn deposit_lp_token(
    lp_token: u64,
    slippage: u8,
    base_reserve: u64,
    quote_reserve: u64,
    total_lp_tokens: u64,
) -> Result<(u64, u64), error::ClientError> {
    if total_lp_tokens == 0 {
        return Err(error::ClientError::OtherError(
            "Total LP tokens cannot be zero".into(),
        ));
    }

    if slippage > 100 {
        return Err(error::ClientError::OtherError(
            "Slippage must be between 0 and 100".into(),
        ));
    }

    let base_amount_in = ((base_reserve as u128).checked_mul(lp_token as u128).ok_or(
        error::ClientError::OtherError("Multiplication overflow".into()),
    )?)
    .div_ceil(total_lp_tokens as u128) as u64;

    let quote_amount_in = ((quote_reserve as u128)
        .checked_mul(lp_token as u128)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?)
    .div_ceil(total_lp_tokens as u128) as u64;

    let slippage_factor = ((1_u128 + (slippage as u128)) * 1_000_000_000) / 100;
    let slippage_denominator = 1_000_000_000;

    let max_base = (base_amount_in as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(slippage_denominator)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    let max_quote = (quote_amount_in as u128)
        .checked_mul(slippage_factor)
        .ok_or(error::ClientError::OtherError(
            "Multiplication overflow".into(),
        ))?
        .checked_div(slippage_denominator)
        .ok_or(error::ClientError::OtherError("Division by zero".into()))?
        as u64;

    Ok((max_base, max_quote))
}
