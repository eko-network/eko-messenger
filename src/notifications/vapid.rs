use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use openssl::bn::BigNumContext;
use openssl::ec::{EcGroup, EcKey, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

fn b64(b: Vec<u8>) -> String {
    URL_SAFE_NO_PAD.encode(b)
}

pub async fn maybe_create_vapid_key(path: &str) -> Result<String> {
    if Path::new(path).exists() {
        // Load existing key and extract public key
        return Ok(b64(load_public_key(path).await.with_context(|| {
            format!("Failed to load existing VAPID key from: {}", path)
        })?));
    }

    Ok(b64(generate_and_save_key(path).await.with_context(
        || format!("Failed to generate and save VAPID key to: {}", path),
    )?))
}

async fn generate_and_save_key(path: &str) -> Result<Vec<u8>> {
    let (pem_bytes, public_key) = tokio::task::spawn_blocking(|| {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let ec_key = EcKey::generate(&group)?;

        // Extract uncompressed public key (65 bytes)
        let mut ctx = BigNumContext::new()?;
        let public_key =
            ec_key
                .public_key()
                .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut ctx)?;

        let pkey = PKey::from_ec_key(ec_key)?;
        let pem = pkey.private_key_to_pem_pkcs8()?;

        Ok::<_, openssl::error::ErrorStack>((pem, public_key))
    })
    .await?
    .context("Failed to generate VAPID key pair")?;

    let parent_dir = Path::new(path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid VAPID key path (no parent directory): {}", path))?;

    if !parent_dir.exists() {
        return Err(anyhow::anyhow!(
            "Parent directory does not exist for VAPID key path: {} (parent: {})",
            path,
            parent_dir.display()
        ));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .with_context(|| format!("Failed to create VAPID key file at: {}", path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file
            .metadata()
            .await
            .with_context(|| format!("Failed to get metadata for VAPID key file: {}", path))?
            .permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(path, perms)
            .await
            .with_context(|| format!("Failed to set permissions for VAPID key file: {}", path))?;
    }

    file.write_all(&pem_bytes)
        .await
        .with_context(|| format!("Failed to write VAPID key to file: {}", path))?;
    Ok(public_key)
}

async fn load_public_key(path: &str) -> Result<Vec<u8>> {
    let path_buf = PathBuf::from(path);
    let path_display = path_buf.display().to_string();
    tokio::task::spawn_blocking(move || {
        let pem = std::fs::read(&path_buf)
            .with_context(|| format!("Failed to read VAPID key file at: {}", path_display))?;
        let pkey =
            PKey::private_key_from_pem(&pem).context("Failed to parse PEM-encoded private key")?;
        let ec_key = pkey
            .ec_key()
            .context("Failed to extract EC key from PKey")?;

        let group = ec_key.group();
        let mut ctx = BigNumContext::new()?;
        let public_key =
            ec_key
                .public_key()
                .to_bytes(group, PointConversionForm::UNCOMPRESSED, &mut ctx)?;

        Ok::<_, anyhow::Error>(public_key)
    })
    .await?
}
