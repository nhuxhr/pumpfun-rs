use crate::error;

pub mod buy;
pub mod deposit;
pub mod sell;
pub mod withdraw;

pub const MAX_FEE_BASIS_POINTS: u64 = 10_000; // 100%

/// Performs ceiling division of a by b
fn ceil_div(a: u64, b: u64) -> Result<u64, error::ClientError> {
    if b == 0 {
        return Err(error::ClientError::OtherError("Division by zero".into()));
    }
    Ok(a.div_ceil(b))
}

/// Calculates fee amount based on the input amount and fee basis points using ceiling division
pub fn fee(amount: u64, fee_bps: u64) -> Result<u64, error::ClientError> {
    amount
        .checked_mul(fee_bps)
        .ok_or(error::ClientError::OtherError(
            "Fee calculation overflow".into(),
        ))
        .and_then(|product| ceil_div(product, MAX_FEE_BASIS_POINTS))
}
