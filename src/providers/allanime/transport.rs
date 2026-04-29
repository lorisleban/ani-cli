use std::process::Command;

use super::config::{ALLANIME_API, ALLANIME_REFR, USER_AGENT};

pub(super) fn is_captcha_response(resp: &str) -> bool {
    resp.contains("NEED_CAPTCHA") || resp.contains("Just a moment") || resp.contains("cf-chl")
}

pub(super) fn curl_post_api(payload: String, label: Option<&str>) -> Result<String, String> {
    let api_url = format!("{}/api", ALLANIME_API);
    let output = Command::new("curl")
        .args([
            "-e",
            ALLANIME_REFR,
            "-s",
            "-H",
            "Content-Type: application/json",
            "-X",
            "POST",
            &api_url,
            "--data",
            &payload,
            "-A",
            USER_AGENT,
        ])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err("curl request failed".to_string());
    }

    let body =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid curl response: {}", e))?;

    if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
        if let Some(label) = label {
            let _ = std::fs::write(format!("/tmp/ani-cli-{}-curl.json", label), &body);
        }
    }

    Ok(body)
}

pub(super) fn curl_get(
    url: &str,
    label: Option<&str>,
    origin: Option<&str>,
) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-e", ALLANIME_REFR, "-s", "-A", USER_AGENT]);
    if let Some(origin) = origin {
        cmd.args(["-H", &format!("Origin: {}", origin)]);
    }
    cmd.arg(url);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err("curl request failed".to_string());
    }

    let body =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid curl response: {}", e))?;

    if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
        if let Some(label) = label {
            let _ = std::fs::write(format!("/tmp/ani-cli-{}-curl.json", label), &body);
        }
    }

    Ok(body)
}
