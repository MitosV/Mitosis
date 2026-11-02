use aes::Aes128;
use cfb8::cipher::KeyIvInit;

pub struct EncryptionCipher {
    enc: cfb8::Encryptor<Aes128>,
    dec: cfb8::Decryptor<Aes128>,
}

impl EncryptionCipher {
    pub fn new(key: &[u8]) -> Self {
        let key_16: &[u8; 16] = key.try_into().expect("Expected 16-byte key");
        let enc = cfb8::Encryptor::new(key_16.into(), key_16.into());
        let dec = cfb8::Decryptor::new(key_16.into(), key_16.into());
        Self { enc, dec }
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        self.enc.encrypt(data);
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        self.dec.decrypt(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let key = [0x42u8; 16];
        let mut cipher = EncryptionCipher::new(&key);
        let original = b"Mitosis Minecraft Headless Bot Engine";
        let mut buf = original.to_vec();

        cipher.encrypt(&mut buf);
        assert_ne!(&buf[..], original);

        cipher.decrypt(&mut buf);
        assert_eq!(&buf[..], original);
    }
}
