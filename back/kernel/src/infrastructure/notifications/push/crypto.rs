use ring::rand::{SecureRandom, SystemRandom};
use ring::{aead, agreement, hkdf};

use crate::errors::AppError;

pub(crate) const RECORD_SIZE: u32 = 4096;
pub(crate) const SALT_LENGTH: usize = 16;
pub(crate) const MAX_PLAINTEXT_LENGTH: usize = 3800;

struct OutputLength(usize);

impl hkdf::KeyType for OutputLength {
    fn len(&self) -> usize {
        self.0
    }
}

fn crypto_error(_: ring::error::Unspecified) -> AppError {
    AppError::Internal("web push encryption failed".to_owned())
}

fn hkdf_expand(salt: &[u8], ikm: &[u8], info: &[u8], length: usize) -> Result<Vec<u8>, AppError> {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(ikm);
    let info = [info];
    let okm = prk
        .expand(&info, OutputLength(length))
        .map_err(crypto_error)?;
    let mut output = vec![0; length];
    okm.fill(&mut output).map_err(crypto_error)?;
    Ok(output)
}

pub(crate) fn encrypt_aes128gcm(
    user_agent_public: &[u8],
    auth_secret: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, AppError> {
    if plaintext.len() > MAX_PLAINTEXT_LENGTH {
        return Err(AppError::Internal(
            "web push payload is too large".to_owned(),
        ));
    }
    let rng = SystemRandom::new();
    let private = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
        .map_err(crypto_error)?;
    let server_public = private.compute_public_key().map_err(crypto_error)?;
    let peer = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, user_agent_public);
    let shared_secret = agreement::agree_ephemeral(private, &peer, |secret| secret.to_vec())
        .map_err(crypto_error)?;

    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(user_agent_public);
    key_info.extend_from_slice(server_public.as_ref());
    let ikm = hkdf_expand(auth_secret, &shared_secret, &key_info, 32)?;

    let mut salt = [0u8; SALT_LENGTH];
    rng.fill(&mut salt).map_err(crypto_error)?;
    let content_key = hkdf_expand(&salt, &ikm, b"Content-Encoding: aes128gcm\0", 16)?;
    let nonce = hkdf_expand(&salt, &ikm, b"Content-Encoding: nonce\0", 12)?;

    let mut record = plaintext.to_vec();
    record.push(0x02);
    let key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_128_GCM, &content_key).map_err(crypto_error)?,
    );
    key.seal_in_place_append_tag(
        aead::Nonce::try_assume_unique_for_key(&nonce).map_err(crypto_error)?,
        aead::Aad::empty(),
        &mut record,
    )
    .map_err(crypto_error)?;

    let server_public = server_public.as_ref();
    let mut body = Vec::with_capacity(SALT_LENGTH + 5 + server_public.len() + record.len());
    body.extend_from_slice(&salt);
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(
        u8::try_from(server_public.len()).map_err(|_| crypto_error(ring::error::Unspecified))?,
    );
    body.extend_from_slice(server_public);
    body.extend_from_slice(&record);
    Ok(body)
}
