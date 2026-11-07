use crate::{
    manifest::{PluginManifest, SETTING_SIGNING_IDENTITY, SETTING_TLS_IDENTITY},
    registry::LoadedPlugin,
    runtime::wit,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::domain::access::AccessScope;
use openssl::{
    hash::MessageDigest,
    pkey::{Id, PKey, Private},
    rsa::Padding,
    sign::Signer,
    x509::X509,
};

pub const ALGORITHMS: &[&str] = &[
    "rsa_pkcs1_sha256",
    "rsa_pkcs1_sha512",
    "rsa_pkcs1_sha1",
    "ed25519",
];
const MAX_SIGN_BYTES: usize = 1024 * 1024;

fn invalid() -> AppError {
    AppError::Validation("invalid signing identity, key type or algorithm".to_owned())
}

fn material(pem: &str) -> Result<(PKey<Private>, Vec<X509>), AppError> {
    let key = PKey::private_key_from_pem(pem.as_bytes()).map_err(|_| invalid())?;
    let certificates = X509::stack_from_pem(pem.as_bytes()).map_err(|_| invalid())?;
    let leaf = certificates.first().ok_or_else(invalid)?;
    if !leaf.public_key().map_err(|_| invalid())?.public_eq(&key) {
        return Err(invalid());
    }
    Ok((key, certificates))
}

fn algorithm_key_matches(key: &PKey<Private>, algorithm: &str) -> bool {
    match algorithm {
        "rsa_pkcs1_sha256" | "rsa_pkcs1_sha512" | "rsa_pkcs1_sha1" => key.id() == Id::RSA,
        "ed25519" => key.id() == Id::ED25519,
        _ => false,
    }
}

pub fn validate_identity(pem: &str, algorithms: &[String]) -> Result<(), AppError> {
    let (key, _) = material(pem)?;
    if algorithms
        .iter()
        .any(|algorithm| !algorithm_key_matches(&key, algorithm))
    {
        return Err(invalid());
    }
    Ok(())
}

fn sign_material(pem: &str, algorithm: &str, data: &[u8]) -> Result<wit::Signature, AppError> {
    let (key, certificates) = material(pem)?;
    if !algorithm_key_matches(&key, algorithm) {
        return Err(invalid());
    }
    let value = if algorithm == "ed25519" {
        Signer::new_without_digest(&key)
            .map_err(|_| invalid())?
            .sign_oneshot_to_vec(data)
            .map_err(|_| invalid())?
    } else {
        let digest = match algorithm {
            "rsa_pkcs1_sha256" => MessageDigest::sha256(),
            "rsa_pkcs1_sha512" => MessageDigest::sha512(),
            // Legacy protocols only; never implicitly selected.
            "rsa_pkcs1_sha1" => MessageDigest::sha1(),
            _ => return Err(invalid()),
        };
        let mut signer = Signer::new(digest, &key).map_err(|_| invalid())?;
        signer
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|_| invalid())?;
        signer.update(data).map_err(|_| invalid())?;
        signer.sign_to_vec().map_err(|_| invalid())?
    };
    let certificates = certificates
        .iter()
        .map(|cert| cert.to_der().map_err(|_| invalid()))
        .collect::<Result<_, _>>()?;
    Ok(wit::Signature {
        value,
        certificates,
    })
}

fn authorized<'a>(
    manifest: &'a PluginManifest,
    identity: &str,
    algorithm: &str,
) -> Result<&'a crate::manifest::SettingSpec, AppError> {
    manifest
        .settings
        .iter()
        .find(|setting| {
            setting.key == identity
                && matches!(
                    setting.kind.as_str(),
                    SETTING_TLS_IDENTITY | SETTING_SIGNING_IDENTITY
                )
                && setting
                    .signing_algorithms
                    .iter()
                    .any(|allowed| allowed == algorithm)
        })
        .ok_or_else(|| {
            AppError::forbidden_code(
                "plugin_signing_not_allowed",
                "identity or signing algorithm is not declared",
            )
        })
}

pub async fn sign(
    plugin: &LoadedPlugin,
    identity: &str,
    algorithm: &str,
    data: Vec<u8>,
) -> Result<wit::Signature, AppError> {
    if data.len() > MAX_SIGN_BYTES {
        return Err(AppError::Validation(
            "signing input exceeds 1 MiB".to_owned(),
        ));
    }
    let setting = authorized(&plugin.manifest, identity, algorithm)?;
    let pem = aphel_sdk::runtime()?
        .secrets()
        .reveal(
            &AccessScope::system(),
            &crate::host::setting_secret_key(&plugin.key, setting),
        )
        .await?
        .ok_or_else(|| AppError::Configuration("signing identity is not configured".to_owned()))?;
    let algorithm = algorithm.to_owned();
    // Bound concurrent private-key operations; do not block an async worker with RSA.
    static CAPACITY: std::sync::LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(tokio::sync::Semaphore::new(4)));
    let permit = CAPACITY
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| invalid())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        sign_material(pem.expose(), &algorithm, &data)
    })
    .await
    .map_err(|_| AppError::Internal("signing task failed".to_owned()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::sign::Verifier;
    const PEM: &str = include_str!("../tests/fixtures/mtls/client-identity.pem");
    #[test]
    fn signatures_verify_and_detect_tampering() {
        let (_, certs) = material(PEM).unwrap();
        let public = certs[0].public_key().unwrap();
        for (algorithm, digest) in [
            ("rsa_pkcs1_sha256", MessageDigest::sha256()),
            ("rsa_pkcs1_sha512", MessageDigest::sha512()),
            ("rsa_pkcs1_sha1", MessageDigest::sha1()),
        ] {
            let signature = sign_material(PEM, algorithm, b"canonical document").unwrap();
            let mut verifier = Verifier::new(digest, &public).unwrap();
            verifier.update(b"canonical document").unwrap();
            assert!(verifier.verify(&signature.value).unwrap());
            let mut verifier = Verifier::new(digest, &public).unwrap();
            verifier.update(b"changed document").unwrap();
            assert!(!verifier.verify(&signature.value).unwrap());
            assert_eq!(signature.certificates[0], certs[0].to_der().unwrap());
            assert_eq!(
                signature.value,
                sign_material(PEM, algorithm, b"canonical document")
                    .unwrap()
                    .value
            );
        }
        assert!(sign_material(PEM, "ed25519", b"wrong key").is_err());
        assert!(sign_material(PEM, "unknown", b"document").is_err());
    }
    #[test]
    fn signing_is_explicitly_opted_in() {
        let mut manifest: PluginManifest = serde_json::from_value(serde_json::json!({"key":"test","name":"Test","version":"1.0.0","api":1,
            "settings":[{"key":"identity","label":"Identity","type":"tls_identity","tls_origins":["https://example.com"]}]})).unwrap();
        assert!(authorized(&manifest, "identity", "rsa_pkcs1_sha256").is_err());
        manifest.settings[0]
            .signing_algorithms
            .push("rsa_pkcs1_sha256".into());
        assert!(authorized(&manifest, "identity", "rsa_pkcs1_sha256").is_ok());
        assert!(authorized(&manifest, "identity", "rsa_pkcs1_sha1").is_err());
        assert!(authorized(&manifest, "another", "rsa_pkcs1_sha256").is_err());
    }
}
