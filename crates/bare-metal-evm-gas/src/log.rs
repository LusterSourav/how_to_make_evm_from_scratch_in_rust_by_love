use crate::constants::{LOG_DATA_GAS, LOG_GAS, LOG_TOPIC_GAS};
use crate::error::GasError;

/// Compute gas cost for a LOG opcode.
///
/// Cost = 375 (base) + 375 * num_topics + 8 * data_len
pub fn log_gas(num_topics: u8, data_len: u64) -> Result<u64, GasError> {
    let gas = LOG_GAS
        .checked_add(
            LOG_TOPIC_GAS
                .checked_mul(num_topics as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)?;
    let gas = gas
        .checked_add(
            LOG_DATA_GAS
                .checked_mul(data_len)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)?;
    Ok(gas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_zero_topics_zero_data() {
        assert_eq!(log_gas(0, 0).unwrap(), 375);
    }

    #[test]
    fn log_one_topic() {
        assert_eq!(log_gas(1, 0).unwrap(), 375 + 375);
    }

    #[test]
    fn log_four_topics() {
        assert_eq!(log_gas(4, 0).unwrap(), 375 + 4 * 375);
    }

    #[test]
    fn log_with_data() {
        assert_eq!(log_gas(0, 100).unwrap(), 375 + 8 * 100);
    }

    #[test]
    fn log_full() {
        assert_eq!(log_gas(4, 100).unwrap(), 375 + 4 * 375 + 8 * 100);
    }
}
