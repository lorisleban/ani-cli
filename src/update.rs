use std::fs::{self, File};
use std::path::{Path, PathBuf};

use reqwest::header::{ACCEPT, USER_AGENT};
use self_update::{ArchiveKind, Download, Extract};
use semver::Version;
use serde::Deserialize;

const REPO_OWNER: &str = "lorisleban";
const REPO_NAME: &str = "ani-cli";
const APP_USER_AGENT: &str = concat!("ani-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct ReleaseSummary {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub update_available: bool,
}

#[derive(Debug)]
pub enum UpgradeOutcome {
    UpToDate { version: String },
    Updated { version: String },
    UsePackageManager { message: String },
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMethod {
    Cargo,
    Homebrew,
    Scoop,
    Winget,
}

impl InstallMethod {
    fn guidance(self) -> &'static str {
        match self {
            InstallMethod::Cargo => {
                "This install looks Cargo-managed. Upgrade with: cargo install --git https://github.com/lorisleban/ani-cli"
            }
            InstallMethod::Homebrew => {
                "This install looks Homebrew-managed. Upgrade with: brew upgrade ani-cli"
            }
            InstallMethod::Scoop => {
                "This install looks Scoop-managed. Upgrade with the package manager command you used to install ani-cli."
            }
            InstallMethod::Winget => {
                "This install looks WinGet-managed. Upgrade with the package manager command you used to install ani-cli."
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ReleaseAssetSpec {
    asset_name: &'static str,
    binary_name: &'static str,
    archive_kind: ArchiveKind,
}

pub async fn check_for_update() -> Result<ReleaseSummary, String> {
    let release = fetch_latest_release().await?;
    let current = current_version()?;
    let latest = parse_version_tag(&release.tag_name)?;

    Ok(ReleaseSummary {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        release_url: release.html_url,
        update_available: latest > current,
    })
}

pub async fn upgrade() -> Result<UpgradeOutcome, String> {
    if let Some(method) = detect_install_method(&current_exe_path()?) {
        return Ok(UpgradeOutcome::UsePackageManager {
            message: method.guidance().to_string(),
        });
    }

    let release = fetch_latest_release().await?;
    let current = current_version()?;
    let latest = parse_version_tag(&release.tag_name)?;
    if latest <= current {
        return Ok(UpgradeOutcome::UpToDate {
            version: current.to_string(),
        });
    }

    let spec = current_release_asset()?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == spec.asset_name)
        .ok_or_else(|| format!("latest release is missing asset {}", spec.asset_name))?;

    let temp_dir = temp_upgrade_dir()?;
    let archive_path = temp_dir.join(spec.asset_name);
    let archive_file =
        File::create(&archive_path).map_err(|err| format!("create temp archive: {err}"))?;

    Download::from_url(&asset.browser_download_url)
        .set_header(
            ACCEPT,
            "application/octet-stream"
                .parse()
                .map_err(|err| format!("accept header: {err}"))?,
        )
        .download_to(&archive_file)
        .map_err(|err| format!("download update: {err}"))?;

    let extract_target = PathBuf::from(spec.binary_name);
    Extract::from_source(&archive_path)
        .archive(spec.archive_kind)
        .extract_file(&temp_dir, &extract_target)
        .map_err(|err| format!("extract update: {err}"))?;

    let new_exe = temp_dir.join(spec.binary_name);
    self_update::self_replace::self_replace(new_exe)
        .map_err(|err| format!("replace current executable: {err}"))?;

    Ok(UpgradeOutcome::Updated {
        version: latest.to_string(),
    })
}

fn current_version() -> Result<Version, String> {
    Version::parse(env!("CARGO_PKG_VERSION")).map_err(|err| format!("parse current version: {err}"))
}

async fn fetch_latest_release() -> Result<GitHubRelease, String> {
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .map_err(|err| format!("create update client: {err}"))?;

    client
        .get(format!(
            "https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest"
        ))
        .header(USER_AGENT, APP_USER_AGENT)
        .send()
        .await
        .map_err(|err| format!("request latest release: {err}"))?
        .error_for_status()
        .map_err(|err| format!("latest release response: {err}"))?
        .json::<GitHubRelease>()
        .await
        .map_err(|err| format!("decode latest release: {err}"))
}

fn parse_version_tag(tag: &str) -> Result<Version, String> {
    Version::parse(tag.trim_start_matches('v'))
        .map_err(|err| format!("parse release tag {tag}: {err}"))
}

fn current_release_asset() -> Result<ReleaseAssetSpec, String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-linux-x86_64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        })
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-macos-x86_64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        })
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-macos-aarch64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        })
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-windows-x86_64.zip",
            binary_name: "ani-cli.exe",
            archive_kind: ArchiveKind::Zip,
        })
    }

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        Err("self-upgrade is not supported on this platform yet".to_string())
    }
}

fn current_exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|err| format!("current executable path: {err}"))
}

fn temp_upgrade_dir() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!(
        "ani-cli-upgrade-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::create_dir_all(&path).map_err(|err| format!("create temp dir: {err}"))?;
    Ok(path)
}

fn detect_install_method(path: &Path) -> Option<InstallMethod> {
    let lowered = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    if lowered.contains("/.cargo/bin/ani-cli") {
        return Some(InstallMethod::Cargo);
    }
    if lowered.contains("/cellar/ani-cli/") || lowered.contains("/homebrew/") {
        return Some(InstallMethod::Homebrew);
    }
    if lowered.contains("/scoop/apps/ani-cli/")
        || lowered.contains("/scoop/shims/ani-cli")
        || lowered.contains("/scoop/apps/ani-cli/current/")
    {
        return Some(InstallMethod::Scoop);
    }
    if lowered.contains("/windowsapps/") {
        return Some(InstallMethod::Winget);
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{detect_install_method, parse_version_tag, InstallMethod};

    #[test]
    fn parses_release_tags_with_leading_v() {
        let version = parse_version_tag("v1.2.3").expect("version");
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn detects_cargo_install_path() {
        let method = detect_install_method(Path::new("/home/user/.cargo/bin/ani-cli"));
        assert_eq!(method, Some(InstallMethod::Cargo));
    }

    #[test]
    fn detects_scoop_install_path() {
        let method = detect_install_method(Path::new(
            r"C:\Users\loris\scoop\apps\ani-cli\current\ani-cli.exe",
        ));
        assert_eq!(method, Some(InstallMethod::Scoop));
    }
}
