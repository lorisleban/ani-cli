use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub use crate::domain::playback::PlayerType;

impl PlayerType {
    pub fn detect() -> Self {
        if cfg!(target_os = "macos") && iina_cli_path().is_some() {
            return PlayerType::Iina;
        }
        if player_command(PlayerType::Mpv).is_some() {
            return PlayerType::Mpv;
        }
        if player_command(PlayerType::Vlc).is_some() {
            return PlayerType::Vlc;
        }
        PlayerType::Mpv
    }
}

pub fn launch_player(
    player: PlayerType,
    url: &str,
    title: &str,
    referer: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), String> {
    match player {
        PlayerType::Iina => launch_iina(url, title, referer, subtitle),
        PlayerType::Mpv => launch_mpv(url, title, referer, subtitle),
        PlayerType::Vlc => launch_vlc(url, title, referer),
    }
}

fn launch_iina(
    url: &str,
    title: &str,
    referer: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let iina_cli = iina_cli_path().ok_or_else(|| "IINA not found".to_string())?;
        let mut cmd = Command::new(iina_cli);
        cmd.arg("--no-stdin")
            .arg(format!("--mpv-force-media-title={title}"));

        if let Some(refr) = referer {
            cmd.arg(format!("--mpv-referrer={refr}"));
        }
        if let Some(sub) = subtitle {
            cmd.arg(format!("--mpv-sub-file={sub}"));
        }
        if !is_iina_running() {
            cmd.arg("--keep-running");
        }
        cmd.arg(url);

        spawn_detached(&mut cmd).map_err(|e| format!("Failed to launch iina: {e}"))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (url, title, referer, subtitle);
        Err("IINA is only supported on macOS".to_string())
    }
}

fn launch_mpv(
    url: &str,
    title: &str,
    referer: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), String> {
    let player = player_command(PlayerType::Mpv).ok_or_else(|| "mpv not found".to_string())?;
    let mut cmd = Command::new(player);
    cmd.arg(format!("--force-media-title={title}"));

    if let Some(refr) = referer {
        cmd.arg(format!("--referrer={refr}"));
    }
    if let Some(sub) = subtitle {
        cmd.arg(format!("--sub-file={sub}"));
    }
    cmd.arg(url);

    spawn_detached(&mut cmd).map_err(|e| format!("Failed to launch mpv: {e}"))?;
    Ok(())
}

fn launch_vlc(url: &str, title: &str, referer: Option<&str>) -> Result<(), String> {
    let player = player_command(PlayerType::Vlc).ok_or_else(|| "VLC not found".to_string())?;
    let mut cmd = Command::new(player);
    cmd.arg("--play-and-exit")
        .arg(format!("--meta-title={title}"));

    if let Some(refr) = referer {
        cmd.arg(format!("--http-referrer={refr}"));
    }
    cmd.arg(url);

    spawn_detached(&mut cmd).map_err(|e| format!("Failed to launch vlc: {e}"))?;
    Ok(())
}

fn player_command(player: PlayerType) -> Option<PathBuf> {
    match player {
        PlayerType::Iina => iina_cli_path(),
        PlayerType::Mpv => {
            find_in_path(&player_binary_candidates(player)).or_else(|| windows_player_path(player))
        }
        PlayerType::Vlc => {
            find_in_path(&player_binary_candidates(player)).or_else(|| windows_player_path(player))
        }
    }
}

pub fn detected_player_path(player: PlayerType) -> Option<PathBuf> {
    player_command(player)
}

fn player_binary_candidates(player: PlayerType) -> Vec<&'static str> {
    match player {
        PlayerType::Mpv => {
            #[cfg(windows)]
            {
                vec!["mpv.exe", "mpv.com", "mpv"]
            }
            #[cfg(not(windows))]
            {
                vec!["mpv"]
            }
        }
        PlayerType::Vlc => {
            #[cfg(windows)]
            {
                vec!["vlc.exe", "vlc"]
            }
            #[cfg(not(windows))]
            {
                vec!["vlc"]
            }
        }
        PlayerType::Iina => vec!["iina-cli"],
    }
}

fn find_in_path(candidates: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    for dir in std::env::split_paths(&path_var) {
        for candidate in candidates {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn iina_cli_path() -> Option<PathBuf> {
    let path = PathBuf::from("/Applications/IINA.app/Contents/MacOS/iina-cli");
    path.is_file().then_some(path)
}

#[cfg(not(target_os = "macos"))]
fn iina_cli_path() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "macos")]
fn is_iina_running() -> bool {
    Command::new("pgrep")
        .arg("-f")
        .arg("IINA")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn is_iina_running() -> bool {
    false
}

#[cfg(windows)]
fn windows_player_path(player: PlayerType) -> Option<PathBuf> {
    let names = match player {
        PlayerType::Mpv => ["mpv.exe", "mpv.com"].as_slice(),
        PlayerType::Vlc => ["vlc.exe"].as_slice(),
        PlayerType::Iina => return None,
    };

    let mut roots = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        let path = PathBuf::from(program_files_x86);
        if !roots.contains(&path) {
            roots.push(path);
        }
    }
    if let Some(local_app_data) = std::env::var_os("LocalAppData") {
        roots.push(PathBuf::from(local_app_data));
    }

    let subdirs: &[&str] = match player {
        PlayerType::Mpv => &["mpv", "mpv.net", "Programs\\mpv"],
        PlayerType::Vlc => &["VideoLAN\\VLC", "VLC"],
        PlayerType::Iina => &[],
    };

    for root in roots {
        for subdir in subdirs {
            let base = root.join(subdir);
            for name in names {
                let candidate = base.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn windows_player_path(_player: PlayerType) -> Option<PathBuf> {
    None
}

fn spawn_detached(cmd: &mut Command) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn()?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        // Keep Unix detached semantics close to the original nohup-based flow.
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        cmd.spawn()?;
        Ok(())
    }
}
