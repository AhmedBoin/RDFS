use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn verify_signature(public_key: &[u8; 32], signature_bytes: &[u8; 64], message: &[u8]) -> bool {
    let verifying_key = match VerifyingKey::from_bytes(public_key) {
        Ok(key) => key,
        Err(_) => return false,
    };

    let signature = Signature::from_bytes(signature_bytes);

    verifying_key.verify(message, &signature).is_ok()
}

pub fn sign_message(private_key: &[u8; 32], message: &[u8]) -> [u8; 64] {
    let signing_key = SigningKey::from_bytes(private_key);
    let signature: Signature = signing_key.sign(message);
    signature.to_bytes()
}

pub fn verify_bytes(public_key: &[u8; 32], bytes: &[u8]) -> bool {
    if bytes.len() < 64 {
        return false;
    }
    let length = bytes.len() - 64;
    verify_signature(public_key, bytes[length..].try_into().unwrap(), &bytes[..length])
}

pub fn sign_bytes(private_key: &[u8; 32], bytes: &mut [u8]) {
    if bytes.len() < 64 {
        return;
    }
    let length = bytes.len() - 64;
    let signature = sign_message(private_key, &bytes[..length]);
    bytes[length..].copy_from_slice(&signature);
}

#[cfg(test)]
mod test {
    use super::*;
    use ed25519_dalek::{SigningKey, VerifyingKey};

    #[test]
    fn test_sign_and_verify_valid_signature() {
        let bytes = [0u8; 32];
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = VerifyingKey::from(&signing_key);

        let message = b"this is a test message";

        let private_bytes: [u8; 32] = signing_key.to_bytes();
        let public_bytes: [u8; 32] = verifying_key.to_bytes();

        let signature = sign_message(&private_bytes, message);
        let valid = verify_signature(&public_bytes, &signature, message);

        assert!(valid, "Signature should be valid");
    }

    #[test]
    fn test_invalid_signature_fails_verification() {
        let bytes = [0u8; 32];
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = VerifyingKey::from(&signing_key);

        let message = b"original message";
        let wrong_message = b"tampered message";

        let private_bytes = signing_key.to_bytes();
        let public_bytes = verifying_key.to_bytes();

        let signature = sign_message(&private_bytes, message);

        // Should fail because message was changed
        let valid = verify_signature(&public_bytes, &signature, wrong_message);
        assert!(!valid, "Tampered message should not verify");
    }

    #[test]
    fn test_invalid_key_fails_verification() {
        let bytes1 = [0u8; 32];
        let bytes2 = [255u8; 32];
        let key1 = SigningKey::from_bytes(&bytes1);
        let key2 = SigningKey::from_bytes(&bytes2); // Different key

        let message = b"message signed with key1";

        let signature = sign_message(&key1.to_bytes(), message);

        let valid = verify_signature(&VerifyingKey::from(&key2).to_bytes(), &signature, message);
        assert!(!valid, "Verification with wrong key should fail");
    }
}
