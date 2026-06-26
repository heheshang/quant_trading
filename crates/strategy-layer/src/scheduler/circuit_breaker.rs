//! Circuit Breaker — 熔断保护器
//!
//! 追踪连续错误次数，在达到阈值后熔断，防止故障发散。

use std::time::{Duration, Instant};

/// 熔断保护器
///
/// 当连续错误次数达到 `threshold` 时标记为熔断状态。
/// 熔断后经过 `window_secs` 时间自动恢复。
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// 熔断阈值（连续错误次数）
    threshold: u32,
    /// 熔断窗口（秒）
    window_secs: u64,
    /// 当前连续错误计数
    error_count: u32,
    /// 上次错误时间
    last_error_at: Option<Instant>,
    /// 是否处于熔断状态
    tripped: bool,
    /// 熔断触发时间
    tripped_at: Option<Instant>,
}

impl CircuitBreaker {
    /// 创建新的熔断器
    #[must_use]
    pub fn new(threshold: u32, window_secs: u64) -> Self {
        Self {
            threshold,
            window_secs,
            error_count: 0,
            last_error_at: None,
            tripped: false,
            tripped_at: None,
        }
    }

    /// 记录一次错误，返回是否达到熔断阈值
    pub fn record_error(&mut self) -> bool {
        self.error_count += 1;
        self.last_error_at = Some(Instant::now());

        if self.error_count >= self.threshold && !self.tripped {
            self.tripped = true;
            self.tripped_at = Some(Instant::now());
            return true;
        }
        false
    }

    /// 检查是否处于熔断状态
    ///
    /// 如果熔断已超过窗口期则自动恢复。
    #[must_use]
    pub fn is_tripped(&self) -> bool {
        if !self.tripped {
            return false;
        }
        // 检查是否过了恢复窗口
        if let Some(tripped_at) = self.tripped_at {
            if tripped_at.elapsed() >= Duration::from_secs(self.window_secs) {
                // 已过窗口期，自动恢复（但 self 是 &，实际恢复在调用侧）
                return false;
            }
        }
        true
    }

    /// 重置熔断器（成功执行后调用）
    pub fn reset(&mut self) {
        self.error_count = 0;
        self.last_error_at = None;
        self.tripped = false;
        self.tripped_at = None;
    }

    /// 获取当前错误计数
    #[must_use]
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// 获取熔断阈值
    #[must_use]
    pub fn threshold(&self) -> u32 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_not_tripped() {
        let cb = CircuitBreaker::new(3, 60);
        assert!(!cb.is_tripped());
        assert_eq!(cb.error_count(), 0);
    }

    #[test]
    fn test_records_errors() {
        let mut cb = CircuitBreaker::new(3, 60);
        assert!(!cb.record_error());
        assert!(!cb.record_error());
        assert!(cb.record_error()); // 第三次触发熔断
        assert!(cb.is_tripped());
    }

    #[test]
    fn test_not_tripped_below_threshold() {
        let mut cb = CircuitBreaker::new(5, 60);
        for _ in 0..4 {
            cb.record_error();
        }
        assert!(!cb.is_tripped());
    }

    #[test]
    fn test_reset() {
        let mut cb = CircuitBreaker::new(3, 60);
        cb.record_error();
        cb.record_error();
        cb.record_error();
        assert!(cb.is_tripped());
        cb.reset();
        assert!(!cb.is_tripped());
        assert_eq!(cb.error_count(), 0);
    }

    #[test]
    fn test_auto_recovery_after_window() {
        let mut cb = CircuitBreaker::new(1, 0); // 0秒窗口 = 立即恢复
        cb.record_error();
        // 等待足够时间后，is_tripped 应返回 false（窗口期已过）
        // 但由于 Instant 不可控，我们用 0 秒窗口测试逻辑
        // 0 秒窗口意味着 elapsed() >= 0 总是成立
        assert!(!cb.is_tripped()); // 0秒窗口已自动恢复
    }
}
