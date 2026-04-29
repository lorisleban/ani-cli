use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerType {
    Mpv,
    Iina,
    Vlc,
}

impl PlayerType {
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        {
            if std::path::Path::new("/Applications/IINA.app/Contents/MacOS/iina-cli").exists() {
                return PlayerType::Iina;
            }
        }
        if which("mpv") {
            return PlayerType::Mpv;
        }
        if which("vlc") {
            return PlayerType::Vlc;
        }
        PlayerType::Mpv // default fallback
    }

    pub fn name(&self) -> &str {
        match self {
            PlayerType::Mpv => "mpv",
            PlayerType::Iina => "iina",
            PlayerType::Vlc => "vlc",
        }
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if IINA is already running (to omit --keep-running per the shell script)
fn is_iina_running() -> bool {
    Command::new("pgrep")
        .arg("-f")
        .arg("IINA")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn launch_player(
    player: PlayerType,
    url: &str,
    title: &str,
    referer: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), String> {
    // Use nohup via sh -c to fully detach the player process,
    // exactly like the original shell script does.
    match player {
        PlayerType::Iina => {
            let iina_cli = "/Applications/IINA.app/Contents/MacOS/iina-cli";
            let mut args = vec![
                "--no-stdin".to_string(),
                format!("--mpv-force-media-title={}", shell_escape(title)),
            ];
            if let Some(refr) = referer {
                args.push(format!("--mpv-referrer={}", shell_escape(refr)));
            }
            if let Some(sub) = subtitle {
                args.push(format!("--mpv-sub-file={}", shell_escape(sub)));
            }
            if !is_iina_running() {
                args.push("--keep-running".to_string());
            }
            args.push(shell_escape(url));

            let shell_cmd = format!("nohup '{}' {} >/dev/null 2>&1 &", iina_cli, args.join(" "));

            Command::new("sh")
                .arg("-c")
                .arg(&shell_cmd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch iina: {}", e))?;
        }
        PlayerType::Mpv => {
            let mut args = vec![format!("--force-media-title={}", shell_escape(title))];
            if let Some(refr) = referer {
                args.push(format!("--referrer={}", shell_escape(refr)));
            }
            if let Some(sub) = subtitle {
                args.push(format!("--sub-file={}", shell_escape(sub)));
            }
            args.push(shell_escape(url));

            let shell_cmd = format!("nohup mpv {} >/dev/null 2>&1 &", args.join(" "));

            Command::new("sh")
                .arg("-c")
                .arg(&shell_cmd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch mpv: {}", e))?;
        }
        PlayerType::Vlc => {
            let mut args = vec![
                "--play-and-exit".to_string(),
                format!("--meta-title={}", shell_escape(title)),
            ];
            if let Some(refr) = referer {
                args.push(format!("--http-referrer={}", shell_escape(refr)));
            }
            args.push(shell_escape(url));

            let shell_cmd = format!("nohup vlc {} >/dev/null 2>&1 &", args.join(" "));

            Command::new("sh")
                .arg("-c")
                .arg(&shell_cmd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch vlc: {}", e))?;
        }
    }

    Ok(())
}

/// Shell-escape a string by wrapping in single quotes
fn shell_escape(s: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, start quote)
    format!("'{}'", s.replace('\'', "'\\''"))
}
