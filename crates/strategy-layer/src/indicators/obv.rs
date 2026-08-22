//! 能量潮指标 (On-Balance Volume, OBV)

use rust_decimal::Decimal;
use tracing::instrument;

use super::{IndicatorError, IndicatorResult};

/// 能量潮指标 (OBV)
#[instrument(level = "debug", skip(close, volume), fields(data_len = close.len()))]
pub fn obv(close: &[Decimal], volume: &[Decimal]) -> IndicatorResult<Vec<Decimal>> {
    if close.len() != volume.len() {
        return Err(IndicatorError::InputLengthMismatch {
            reason: format!(
                "close.len()={} != volume.len()={}",
                close.len(),
                volume.len()
            ),
        });
    }

    if close.is_empty() {
        return Ok(vec![]);
    }

    let mut result = Vec::with_capacity(close.len());
    result.push(Decimal::ZERO);

    for i in 1..close.len() {
        let prev = result[i - 1];
        let obv_val = if close[i] > close[i - 1] {
            prev + volume[i]
        } else if close[i] < close[i - 1] {
            prev - volume[i]
        } else {
            prev
        };
        result.push(obv_val);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_obv_basic() {
        let close = vec![dec!(100), dec!(102), dec!(101), dec!(103), dec!(105)];
        let volume = vec![dec!(1000), dec!(1500), dec!(1200), dec!(1800), dec!(2000)];

        let result = obv(&close, &volume).unwrap();
        assert_eq!(result.len(), close.len());
        // OBV[0] is always 0
        assert_eq!(result[0], Decimal::ZERO);
        // close[1]=102 > close[0]=100 → OBV[1] = 0 + 1500 = 1500
        assert_eq!(result[1], dec!(1500));
    }

    #[test]
    fn test_obv_length_mismatch() {
        let close = vec![dec!(100), dec!(102)];
        let volume = vec![dec!(1000)];

        let result = obv(&close, &volume);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndicatorError::InputLengthMismatch { .. } => {}
            e => panic!("Expected InputLengthMismatch, got {:?}", e),
        }
    }

    #[test]
    fn test_obv_flat_close() {
        let close = vec![dec!(100), dec!(100), dec!(100)];
        let volume = vec![dec!(1000), dec!(1500), dec!(1200)];

        let result = obv(&close, &volume).unwrap();
        // When close doesn't change, OBV stays same
        assert_eq!(result[1], result[0]);
        assert_eq!(result[2], result[1]);
    }

    #[test]
    fn test_obv_empty() {
        let result = obv(&[], &[]).unwrap();
        assert!(result.is_empty());
    }
}
