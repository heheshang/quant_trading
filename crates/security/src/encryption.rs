use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use quant_common::{Error, Result};
use rand::RngCore;
use tracing::{error, info, instrument, warn};

/// 数据加密服务
pub struct DataEncryption {
    cipher: Aes256Gcm,
    /// 旧派生（SHA256/截断）的解密器，用于兼容历史数据。
    legacy_cipher: Option<Aes256Gcm>,
}

impl DataEncryption {
    /// 创建新的加密服务
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher, legacy_cipher: None }
    }

    /// 从密钥字符串创建：hex 长密钥解码存满熵；短密钥用 Argon2id 派生；并保留旧派生兼容。
    pub fn from_key_string(key_str: &str) -> Result<Self> {
        let mut s = Self::new(&derive_aes_key(key_str));
        s.legacy_cipher = Some(
            Aes256Gcm::new_from_slice(&derive_legacy_key(key_str))
                .map_err(|e| Error::Internal(format!("Invalid legacy key: {}", e)))?,
        );
        Ok(s)
    }
}

/// 派生 AES-256 密钥：
/// 1. hex 字符串（如 `openssl rand -hex 32`，64 字符）→ 解码取满 32 字节（避免截断丢熵）；
/// 2. 原始字节 ≥32 → 取前 32；
/// 3. 过短 → Argon2id（固定应用盐）派生，避免明文/SHA256 无盐。
fn derive_aes_key(key_str: &str) -> [u8; 32] {
    let trimmed = key_str.trim();
    if let Some(bytes) = decode_hex(trimmed) {
        if bytes.len() >= 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes[..32]);
            return k;
        }
    }
    if trimmed.as_bytes().len() >= 32 {
        let mut k = [0u8; 32];
        k.copy_from_slice(&trimmed.as_bytes()[..32]);
        return k;
    }
    let mut k = [0u8; 32];
    let params = argon2::Params::new(19 * 1024, 2, 1, Some(32)).unwrap_or_default();
    let a2 = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    if a2
        .hash_password_into(trimmed.as_bytes(), b"quant-trading-enc-v1", &mut k)
        .is_err()
    {
        use sha2::{Digest, Sha256};
        k.copy_from_slice(&Sha256::digest(trimmed.as_bytes()));
    }
    k
}

/// 旧派生（升级前）：<32 字节 SHA256 扩展，否则取前 32 字节（用于解密历史数据）。
fn derive_legacy_key(key_str: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let key_bytes = key_str.as_bytes();
    if key_bytes.len() < 32 {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(key_bytes);
        key.copy_from_slice(&hash);
    } else {
        key.copy_from_slice(&key_bytes[..32]);
    }
    key
}

/// 将 hex 字符串解码为字节；非法 hex 或长度非偶数返回 None。
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let sb = s.as_bytes();
    for chunk in sb.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

impl DataEncryption {
    /// 加密数据
    #[instrument(skip(self, plaintext))]
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String> {
        info!("encrypting data");

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher.encrypt(nonce, plaintext).map_err(|e| {
            error!(error = %e, "encryption failed");
            Error::Internal(format!("Encryption failed: {}", e))
        })?;

        // 将 nonce 和密文组合并 base64 编码
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&result))
    }

    /// 解密数据
    #[instrument(skip(self, encrypted))]
    pub fn decrypt(&self, encrypted: &str) -> Result<Vec<u8>> {
        info!("decrypting data");

        let data = general_purpose::STANDARD.decode(encrypted).map_err(|e| {
            error!(error = %e, "base64 decode failed");
            Error::Internal(format!("Base64 decode failed: {}", e))
        })?;

        if data.len() < 12 {
            warn!("invalid encrypted data: too short");
            return Err(Error::Internal("Invalid encrypted data".to_string()));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        match self.cipher.decrypt(nonce, ciphertext) {
            Ok(pt) => Ok(pt),
            Err(e) => {
                // 兼容升级前旧派生密钥加密的数据。
                if let Some(legacy) = &self.legacy_cipher {
                    return legacy
                        .decrypt(nonce, ciphertext)
                        .map_err(|le| {
                            error!(error = %e, legacy = %le, "decryption failed (both ciphers)");
                            Error::Internal("Decryption failed".to_string())
                        });
                }
                error!(error = %e, "decryption failed");
                Err(Error::Internal(format!("Decryption failed: {}", e)))
            }
        }
    }

    /// 加密字符串
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String> {
        self.encrypt(plaintext.as_bytes())
    }

    /// 解密字符串
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String> {
        let bytes = self.decrypt(encrypted)?;
        String::from_utf8(bytes).map_err(|e| Error::Internal(format!("UTF8 decode failed: {}", e)))
    }
}

/// 密码哈希服务
pub struct PasswordHasher;

impl PasswordHasher {
    /// 哈希密码
    #[instrument(skip(password))]
    pub fn hash_password(password: &str) -> Result<String> {
        use argon2::{
            password_hash::{PasswordHasher as _, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                error!(error = %e, "password hashing failed");
                Error::Internal(format!("Password hashing failed: {}", e))
            })?
            .to_string();

        info!("password hashed");
        Ok(password_hash)
    }

    /// 验证密码
    #[instrument(skip(password, hash))]
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };

        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            warn!(error = %e, "invalid password hash format");
            Error::Internal(format!("Invalid hash: {}", e))
        })?;

        let valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        if !valid {
            warn!("password verification failed");
        }

        Ok(valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption() {
        let key = b"my_secret_key_32_bytes_long_here";
        let encryptor = DataEncryption::new(key);

        let plaintext = "Hello, World!";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_password_hashing() {
        let password = "my_secure_password";
        let hash = PasswordHasher::hash_password(password).unwrap();

        assert!(PasswordHasher::verify_password(password, &hash).unwrap());
        assert!(!PasswordHasher::verify_password("wrong_password", &hash).unwrap());
    }
}
