use cargo_packager_updater::{Config, RemoteRelease, Update};
use serde_json::Value;
use std::{io, path::PathBuf, time::Duration};

type Error = Box<dyn std::error::Error + Send + Sync>;
const ENDPOINT: &str = "https://github.com/adeyinkakp/audexis/releases/latest/download/latest.json";
const NORMALIZED_ENDPOINT: &str =
    "https://github.com/adeyinkakp/audexis/releases/latest/download/updater.json";

pub(super) async fn check() -> Result<Option<Update>, Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("Audexis/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client.get(NORMALIZED_ENDPOINT).send().await?;
    let response = if response.status() == reqwest::StatusCode::NOT_FOUND {
        client.get(ENDPOINT).send().await?
    } else {
        response
    };
    let response = response.error_for_status()?;
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(None);
    }
    let manifest: Value = response.json().await?;
    let current: cargo_packager_updater::semver::Version = env!("CARGO_PKG_VERSION").parse()?;
    let version = manifest
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("Update version is missing"))?
        .trim_start_matches('v')
        .parse::<cargo_packager_updater::semver::Version>()?;
    if version <= current {
        return Ok(None);
    }
    let target = cargo_packager_updater::target()
        .ok_or_else(|| io::Error::other("Unsupported update platform"))?;
    let release = parse(manifest, &target)?;
    let download_url = release.download_url(&target)?.clone();
    let signature = release.signature(&target)?.clone();
    let format = release.format(&target)?;
    Ok(Some(Update {
        config: Config {
            endpoints: vec![ENDPOINT.parse()?],
            pubkey: include_str!("../../updater.pub").trim().into(),
            ..Default::default()
        },
        current_version: current.to_string(),
        version: release.version.to_string(),
        date: release.pub_date,
        body: release.notes,
        target,
        extract_path: extract_path()?,
        download_url,
        signature,
        format,
        headers: Default::default(),
        timeout: Some(Duration::from_secs(10 * 60)),
    }))
}

fn parse(mut manifest: Value, target: &str) -> Result<RemoteRelease, Error> {
    if let Some(platforms) = manifest.get_mut("platforms").and_then(Value::as_object_mut) {
        let legacy = target.replacen("macos-", "darwin-", 1);
        let mut platform = platforms
            .get(target)
            .or_else(|| platforms.get(&legacy))
            .cloned()
            .ok_or_else(|| io::Error::other(format!("No update is available for {target}")))?;
        add_format(&mut platform)?;
        platforms.clear();
        platforms.insert(target.into(), platform);
    } else {
        add_format(&mut manifest)?;
    }
    Ok(serde_json::from_value(manifest)?)
}

fn add_format(platform: &mut Value) -> Result<(), Error> {
    if platform.get("format").is_some() {
        return Ok(());
    }
    let url: reqwest::Url = platform
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("Update download URL is missing"))?
        .parse()?;
    let path = url.path().to_ascii_lowercase();
    let format = if path.ends_with(".app.tar.gz") {
        "app"
    } else if path.ends_with(".appimage") {
        "appimage"
    } else if path.ends_with(".exe") {
        "nsis"
    } else if path.ends_with(".msi") {
        "wix"
    } else {
        return Err(io::Error::other(
            "Update format is missing and cannot be determined from its download URL",
        )
        .into());
    };
    platform
        .as_object_mut()
        .ok_or_else(|| io::Error::other("Invalid update entry"))?
        .insert("format".into(), format.into());
    Ok(())
}

fn extract_path() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    if cfg!(target_os = "linux") {
        return Ok(std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .unwrap_or(executable));
    }
    if cfg!(target_os = "macos")
        && let Some(bundle) = executable
            .ancestors()
            .find(|path| path.extension().is_some_and(|ext| ext == "app"))
    {
        return Ok(bundle.to_path_buf());
    }
    executable
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("Cannot locate the installed application"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_packager_updater::UpdateFormat;
    use serde_json::json;

    #[test]
    fn legacy_mac_manifest_keeps_signature_and_maps_platform() {
        let release = parse(json!({"version":"0.20.0","platforms":{
            "darwin-aarch64":{"url":"https://example.com/audexis.app.tar.gz","signature":"original-signature"}
        }}), "macos-aarch64").unwrap();
        assert!(matches!(
            release.format("macos-aarch64").unwrap(),
            UpdateFormat::App
        ));
        assert_eq!(
            release.signature("macos-aarch64").unwrap(),
            "original-signature"
        );
    }

    #[test]
    fn missing_signature_is_rejected() {
        assert!(
            parse(
                json!({"version":"0.20.0","url":"https://example.com/audexis.exe"}),
                "windows-x86_64"
            )
            .is_err()
        );
    }

    #[test]
    fn explicit_format_is_preserved_and_unknown_packages_are_rejected() {
        let release = parse(json!({"version":"0.20.0","url":"https://example.com/download","signature":"signed","format":"nsis"}), "windows-x86_64").unwrap();
        assert!(matches!(
            release.format("windows-x86_64").unwrap(),
            UpdateFormat::Nsis
        ));
        assert!(parse(json!({"version":"0.20.0","url":"https://example.com/download","signature":"signed"}), "windows-x86_64").is_err());
    }
}
