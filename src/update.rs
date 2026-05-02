use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Duration, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use semver::Version;
use serde::Deserialize;
use self_update::{ArchiveKind, Extract};

const OWNER: &str = "lorisleban";
const REPO: &str = "ani-cli";
const UPDATE_INTERVAL_HOURS: i64 = 24;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub release_url: String,
}

#[derive(Debug, Clone)]
pub struct UpdateOutcome {
    pub message: String,
    pub restart_required: bool,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Copy)]
pub enum InstallSource {
    Homebrew,
    Cargo,
    Direct,
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
}

pub fn should_check(last_checked: Option<DateTime<Utc>>) -> bool {
    match last_checked {
        None => true,
        Some(ts) => Utc::now().signed_duration_since(ts) >= Duration::hours(UPDATE_INTERVAL_HOURS),
    }
}

pub async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let release = fetch_latest_release().await?;
    let latest = normalize_tag(&release.tag_name);
    let latest_ver = parse_version(&latest)
        .ok_or_else(|| format!("unable to parse latest version: {}", latest))?;
    let current = current_version();
    if latest_ver > current {
        Ok(Some(UpdateInfo {
            latest_version: latest,
            release_url: release.html_url,
        }))
    } else {
        Ok(None)
    }
}

pub async fn perform_update() -> Result<UpdateOutcome, String> {
    match detect_install_source() {
        InstallSource::Homebrew => run_command("brew", &["upgrade", "ani-cli"])
            .map(|_| UpdateOutcome {
                message: "updated via brew".to_string(),
                restart_required: false,
            }),
        InstallSource::Cargo => run_command(
            "cargo",
            &[
                "install",
                "--git",
                "https://github.com/lorisleban/ani-cli",
                "--force",
            ],
        )
        .map(|_| UpdateOutcome {
            message: "updated via cargo".to_string(),
            restart_required: false,
        }),
        InstallSource::Direct => {
            let release = fetch_latest_release().await?;
            let latest = normalize_tag(&release.tag_name);
            let spec = current_release_asset()?;
            let asset = release
                .assets
                .iter()
                .find(|a| a.name == spec.asset_name)
                .ok_or_else(|| format!("asset not found: {}", spec.asset_name))?;
            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
            let temp_dir = create_temp_dir()?;
            let archive_path = temp_dir.join(&asset.name);
            download_to_path(&asset.browser_download_url, &archive_path).await?;
            let bin_path = extract_archive(&archive_path, &temp_dir, &spec)?;
            let restart_required = replace_binary(&exe_path, &bin_path)?;
            Ok(UpdateOutcome {
                message: if restart_required {
                    format!("update scheduled to v{} — close app to finish", latest)
                } else {
                    format!("updated to v{}", latest)
                },
                restart_required,
            })
        }
    }
}

pub fn detect_install_source() -> InstallSource {
    if let Ok(source) = std::env::var("ANI_CLI_UPDATE_SOURCE") {
        match source.as_str() {
            "brew" => return InstallSource::Homebrew,
            "cargo" => return InstallSource::Cargo,
            "direct" => return InstallSource::Direct,
            _ => {}
        }
    }
    if is_brew_install() {
        return InstallSource::Homebrew;
    }
    if is_cargo_install() {
        return InstallSource::Cargo;
    }
    InstallSource::Direct
}

pub fn open_release_notes(url: &str) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to open release notes".to_string())
    }
}

#[derive(Debug, Clone, Copy)]
struct ReleaseAssetSpec {
    asset_name: &'static str,
    binary_name: &'static str,
    archive_kind: ArchiveKind,
}

async fn fetch_latest_release() -> Result<ReleaseResponse, String> {
    if let Ok(source) = std::env::var("ANI_CLI_UPDATE_URL") {
        return fetch_release_from_source(&source).await;
    }
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        OWNER, REPO
    );
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ani-cli"));
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("update check failed: {}", res.status()));
    }
    res.json::<ReleaseResponse>()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_release_from_source(source: &str) -> Result<ReleaseResponse, String> {
    if let Some(path) = source.strip_prefix("file://") {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        return serde_json::from_slice::<ReleaseResponse>(&data).map_err(|e| e.to_string());
    }
    if Path::new(source).exists() {
        let data = fs::read(source).map_err(|e| e.to_string())?;
        return serde_json::from_slice::<ReleaseResponse>(&data).map_err(|e| e.to_string());
    }
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ani-cli"));
    let client = reqwest::Client::new();
    let res = client
        .get(source)
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("update check failed: {}", res.status()));
    }
    res.json::<ReleaseResponse>()
        .await
        .map_err(|e| e.to_string())
}

fn normalize_tag(tag: &str) -> String {
    tag.trim_start_matches('v').to_string()
}

fn parse_version(tag: &str) -> Option<Version> {
    Version::parse(tag).ok()
}

fn current_release_asset() -> Result<ReleaseAssetSpec, String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-linux-x86_64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-macos-x86_64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-macos-aarch64.tar.gz",
            binary_name: "ani-cli",
            archive_kind: ArchiveKind::Tar(Some(self_update::Compression::Gz)),
        });
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(ReleaseAssetSpec {
            asset_name: "ani-cli-windows-x86_64.zip",
            binary_name: "ani-cli.exe",
            archive_kind: ArchiveKind::Zip,
        });
    }

    #[allow(unreachable_code)]
    Err("self-upgrade is not supported on this platform yet".to_string())
}

fn is_brew_install() -> bool {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let path = exe.to_string_lossy();
    path.contains("/Cellar/") || path.contains("/Homebrew/Cellar/")
}

fn is_cargo_install() -> bool {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let path = exe.to_string_lossy();
    if path.contains("/.cargo/bin/") {
        return true;
    }
    if let Ok(home) = std::env::var("CARGO_HOME") {
        return path.contains(&home);
    }
    false
}

fn run_command(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with status {}", cmd, status))
    }
}

async fn download_to_path(url: &str, path: &Path) -> Result<(), String> {
    if let Ok(asset_path) = std::env::var("ANI_CLI_UPDATE_ASSET_PATH") {
        fs::copy(&asset_path, path).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ani-cli"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/octet-stream"));
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("download failed: {}", res.status()));
    }
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    fs::write(path, &bytes).map_err(|e| e.to_string())
}

fn extract_archive(
    archive_path: &Path,
    temp_dir: &Path,
    spec: &ReleaseAssetSpec,
) -> Result<PathBuf, String> {
    let extract_target = PathBuf::from(spec.binary_name);
    Extract::from_source(archive_path)
        .archive(spec.archive_kind)
        .extract_file(temp_dir, &extract_target)
        .map_err(|err| format!("extract update: {err}"))?;
    let extracted = temp_dir.join(spec.binary_name);
    if extracted.exists() {
        Ok(extracted)
    } else {
        Err("unable to locate binary in archive".to_string())
    }
}

fn replace_binary(target: &Path, source: &Path) -> Result<bool, String> {
    if cfg!(windows) {
        return replace_binary_windows(target, source).map(|_| true);
    }
    let dir = target.parent().ok_or("missing target directory")?;
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid target filename")?;
    let tmp_path = dir.join(format!("{}.new", file_name));
    fs::copy(source, &tmp_path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tmp_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tmp_path, perms).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp_path, target).map_err(|e| e.to_string())?;
    Ok(false)
}

fn replace_binary_windows(target: &Path, source: &Path) -> Result<(), String> {
    let pid = std::process::id();
    let target = target
        .to_str()
        .ok_or("invalid target path")?
        .to_string();
    let source = source
        .to_str()
        .ok_or("invalid source path")?
        .to_string();
    let mut script = std::env::temp_dir();
    script.push(format!("ani-cli-updater-{}.cmd", pid));
    let script_body = format!(
        concat!(
            "@echo off\r\n",
            "set pid={}\r\n",
            "set target=\"{}\"\r\n",
            "set source=\"{}\"\r\n",
            ":wait\r\n",
            "tasklist /FI \"PID eq %pid%\" | find \"%pid%\" >NUL\r\n",
            "if not errorlevel 1 (timeout /T 1 /NOBREAK >NUL & goto wait)\r\n",
            "move /Y %source% %target% >NUL\r\n",
            "start \"\" %target%\r\n",
            "del \"%~f0\"\r\n"
        ),
        pid,
        target,
        source
    );
    fs::write(&script, script_body).map_err(|e| e.to_string())?;
    Command::new("cmd")
        .args(["/C", "start", "", script.to_str().ok_or("invalid script path")?])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn create_temp_dir() -> Result<PathBuf, String> {
    let mut dir = std::env::temp_dir();
    dir.push(format!("ani-cli-update-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
