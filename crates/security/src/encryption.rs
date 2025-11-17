use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use quant_common::{Error, Result};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;

/// 数据加密服务
pub struct DataEncryption {
    cipher: Aes256Gcm,
}

impl DataEncryption {
    /// 创建新的加密服务
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }

    /// 从密钥字符串创建
    pub fn from_key_string(key_str: &str) -> Result<Self> {
        let mut key = [0u8; 32];
        let key_bytes = key_str.as_bytes();
        
        if key_bytes.len() < 32 {
            // 如果密钥太短，用 SHA256 哈希扩展
            use sha2::{Sha256, Digest};
            let hash = Sha256::digest(key_bytes);
            key.copy_from_slice(&hash);
        } else {
            key.copy_from_slice(&key_bytes[..32]);
        }
        
        Ok(Self::new(&key))
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String> {
        // 生成随机 nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| Error::Internal(format!("Encryption failed: {}", e)))?;

        // 将 nonce 和密文组合并 base64 编码
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(general_purpose::STANDARD.encode(&result))
    }

    /// 解密数据
    pub fn decrypt(&self, encrypted: &str) -> Result<Vec<u8>> {
        // Base64 解码
        let data = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| Error::Internal(format!("Base64 decode failed: {}", e)))?;

        if data.len() < 12 {
            return Err(Error::Internal("Invalid encrypted data".to_string()));
        }

        // 分离 nonce 和密文
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        // 解密
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Internal(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    /// 加密字符串
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String> {
        self.encrypt(plaintext.as_bytes())
    }

    /// 解密字符串
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String> {
        let bytes = self.decrypt(encrypted)?;
        String::from_utf8(bytes)
            .map_err(|e| Error::Internal(format!("UTF8 decode failed: {}", e)))
    }
}

/// 密码哈希服务
pub struct PasswordHasher;

impl PasswordHasher {
    /// 哈希密码
    pub fn hash_password(password: &str) -> Result<String> {
        use argon2::{
            password_hash::{PasswordHasher as _, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(format!("Password hashing failed: {}", e)))?
            .to_string();

        Ok(password_hash)
    }

    /// 验证密码
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };

        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| Error::Internal(format!("Invalid hash: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
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
