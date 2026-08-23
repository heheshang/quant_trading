//! RFC 6238 TOTP (Time-based One-Time Passwords) with RFC 4226 HOTP.
//!
//! Uses HMAC-SHA1, a 30-second time step and 6-digit codes — the de-facto
//! standard combination used by Google Authenticator, 1Password, Authy, etc.
//!
//! **Secret format:** random bytes are stored/communicated base32-encoded
//! (RFC 4648, no padding), the canonical TOTP secret representation.

use hmac::{Hmac, Mac};
use quant_common::{Error, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

/// Secret length in bytes (RFC 4226 recommends >= 16; 20 bytes is the common
/// 160-bit strong default across authenticator apps).
const SECRET_LEN: usize = 20;
/// Time step in seconds (RFC 6238 default).
const TIME_STEP_SECS: u64 = 30;
/// Number of returned digits.
const CODE_DIGITS: usize = 6;
/// Default accepted time-step skew on either side (RFC 6238 suggests ±1).
pub const DEFAULT_SKEW: u64 = 1;

/// RFC 4648 base32 alphabet.
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Encode bytes as RFC 4648 base32 (no padding).
fn base32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | u64::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1f) as usize;
            out.push(BASE32_ALPHABET[idx] as char);
        }
        // Keep only the unconsumed low bits so the buffer cannot grow unbounded.
        buffer &= (1u64 << bits) - 1;
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(BASE32_ALPHABET[idx] as char);
    }
    out
}

/// Decode RFC 4648 base32 (case/whitespace tolerant, padding optional) to bytes.
fn base32_decode(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    for c in s.bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        let c = c.to_ascii_uppercase();
        let idx = match c {
            b'A'..=b'Z' => u64::from(c - b'A'),
            b'2'..=b'7' => u64::from(26 + (c - b'2')),
            b'=' => break, // padding terminator
            _ => {
                return Err(Error::Internal(format!(
                    "Invalid base32 character '{}'",
                    c as char
                )))
            }
        };
        buffer = (buffer << 5) | idx;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
            buffer &= (1u64 << bits) - 1;
        }
    }
    Ok(out)
}

/// Generate a random base32 TOTP secret.
///
/// Produces [`SECRET_LEN`] cryptographically-random bytes (from the OS CSPRNG)
/// and returns them RFC-4648 base32-encoded without padding.
pub fn generate_totp_secret() -> Result<String> {
    let mut bytes = vec![0u8; SECRET_LEN];
    OsRng.fill_bytes(&mut bytes);
    Ok(base32_encode(&bytes))
}

/// Decode a base32 secret into raw bytes (case/whitespace tolerant).
fn decode_secret(secret: &str) -> Result<Vec<u8>> {
    let cleaned: String = secret
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    base32_decode(&cleaned)
}

/// Compute the current window counter for `now`.
fn current_time_step() -> u64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs / TIME_STEP_SECS
}

/// Constant-time string comparison (leaks only length equality).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Compute the 6-digit TOTP code for `secret` at the given window counter.
///
/// `time_step` is the HOTP counter value (i.e. `unix_seconds / 30`).
pub fn totp_code(secret: &str, time_step: u64) -> Result<String> {
    let key = decode_secret(secret)?;

    // RFC 4226/6238 counter is an 8-byte big-endian value.
    let counter_bytes = time_step.to_be_bytes();

    let mut mac = HmacSha1::new_from_slice(&key)
        .map_err(|e| Error::Internal(format!("HMAC init failed: {e}")))?;
    mac.update(&counter_bytes);

    let digest = mac.finalize().into_bytes();

    // Dynamic truncation: use the low nibble of the last byte as an offset.
    let offset = usize::from(digest[19] & 0x0f);
    let binary = (u32::from(digest[offset] & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);

    let code = binary % 10u32.pow(CODE_DIGITS as u32);
    Ok(format!("{:0width$}", code, width = CODE_DIGITS))
}

/// Verify a user-supplied code against `secret` for the *current* window.
///
/// `skew` is the number of neighbouring windows accepted on each side
/// (skew = 1 accepts the previous, current and next 30-second windows).
pub fn verify_totp(secret: &str, code: &str, skew: u64) -> bool {
    verify_totp_at_time_step(secret, code, current_time_step(), skew)
}

/// Verify `code` against the code produced for an explicit window counter.
///
/// Useful for tests and for replaying historical windows (e.g. when the clock
/// drifted by more than one step).
pub fn verify_totp_at_time_step(secret: &str, code: &str, time_step: u64, skew: u64) -> bool {
    let code = code.trim();
    if code.len() != CODE_DIGITS || !code.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }

    let expected = match totp_code(secret, time_step) {
        Ok(c) => c,
        Err(_) => return false,
    };
    if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
        return true;
    }

    for offset in 1..=skew {
        let before = match totp_code(secret, time_step.saturating_sub(offset)) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let after = match totp_code(secret, time_step + offset) {
            Ok(c) => c,
            Err(_) => return false,
        };
        if constant_time_eq(before.as_bytes(), code.as_bytes())
            || constant_time_eq(after.as_bytes(), code.as_bytes())
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4226/6238 test secret: base32 of ASCII `"12345678901234567890"`.
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn base32_round_trip() {
        let data = b"12345678901234567890";
        let enc = base32_encode(data);
        assert_eq!(enc, RFC_SECRET);
        assert_eq!(base32_decode(&enc).unwrap(), data);
        // Case/whitespace tolerance.
        assert_eq!(base32_decode(&enc.to_lowercase()).unwrap(), data);
        assert_eq!(
            base32_decode(" GEZD GNBVG Y3TQO JQGEZ DGNBV GY3TQ OJQ ").unwrap(),
            data
        );
    }

    #[test]
    fn generate_secret_is_valid_base32() {
        for _ in 0..8 {
            let secret = generate_totp_secret().unwrap();
            // 20 bytes -> 32 base32 chars (no padding).
            assert_eq!(secret.len(), 32);
            let bytes = base32_decode(&secret).unwrap();
            assert_eq!(bytes.len(), SECRET_LEN);
        }
    }

    #[test]
    fn generated_secrets_are_unique() {
        let s1 = generate_totp_secret().unwrap();
        let s2 = generate_totp_secret().unwrap();
        assert_ne!(s1, s2);
    }

    #[test]
    fn rfc_4226_hotp_sha1_test_vectors_match() {
        // RFC 4226 Appendix D (HMAC-SHA1, secret = base32 of ASCII
        // "12345678901234567890"). We emit 6-digit codes.
        // counter 1 -> 287082 (full 94287082)
        // counter 2 -> 359152
        // counter 3 -> 969429
        // counter 4 -> 338314
        assert_eq!(totp_code(RFC_SECRET, 1).unwrap(), "287082");
        assert_eq!(totp_code(RFC_SECRET, 2).unwrap(), "359152");
        assert_eq!(totp_code(RFC_SECRET, 3).unwrap(), "969429");
        assert_eq!(totp_code(RFC_SECRET, 4).unwrap(), "338314");
    }

    #[test]
    fn totp_is_deterministic_for_same_counter() {
        let s = generate_totp_secret().unwrap();
        let a = totp_code(&s, 12345).unwrap();
        let b = totp_code(&s, 12345).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
    }

    #[test]
    fn totp_changes_with_time_step() {
        let s = generate_totp_secret().unwrap();
        let c1 = totp_code(&s, 100).unwrap();
        let c2 = totp_code(&s, 101).unwrap();
        assert_ne!(c1, c2, "code should vary with time step");
    }

    #[test]
    fn verify_current_code_succeeds() {
        let s = generate_totp_secret().unwrap();
        let step = current_time_step();
        let code = totp_code(&s, step).unwrap();
        assert!(verify_totp(&s, &code, DEFAULT_SKEW));
    }

    #[test]
    fn verify_accepts_neighbouring_window() {
        let s = generate_totp_secret().unwrap();
        // A one-step-old code is still accepted within the ±1 skew window.
        let step = current_time_step();
        let old_code = totp_code(&s, step.saturating_sub(1)).unwrap();
        assert!(verify_totp(&s, &old_code, DEFAULT_SKEW));
    }

    #[test]
    fn verify_rejects_wrong_code() {
        let s = generate_totp_secret().unwrap();
        let code = totp_code(&s, current_time_step()).unwrap();
        let wrong = if code == "000000" { "111111" } else { "000000" };
        assert!(!verify_totp(&s, wrong, DEFAULT_SKEW));
    }

    #[test]
    fn verify_rejects_old_code_outside_skew() {
        let s = generate_totp_secret().unwrap();
        let step = current_time_step();
        // A code 3 steps old is outside the ±1 window.
        let old_code = totp_code(&s, step.saturating_sub(3)).unwrap();
        assert!(!verify_totp(&s, &old_code, DEFAULT_SKEW));
        // ...but is accepted if we widen the skew.
        assert!(verify_totp_at_time_step(&s, &old_code, step, 3));
    }

    #[test]
    fn verify_rejects_malformed_code() {
        let s = generate_totp_secret().unwrap();
        assert!(!verify_totp(&s, "12345", DEFAULT_SKEW));
        assert!(!verify_totp(&s, "abcdef", DEFAULT_SKEW));
        assert!(!verify_totp(&s, "1234567", DEFAULT_SKEW));
    }
}
