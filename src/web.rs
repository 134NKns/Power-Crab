//! Web Portal Module
//!
//! Provides the Captive Portal HTML UI and HTTP request/form handling.

use crate::storage::{WifiCredentials, MAX_PASS_LEN, MAX_SSID_LEN};
use heapless::String;

/// Responsive, modern dark-themed HTML setup page.
/// Embedded directly as a static string — zero external CDN or asset dependencies.
pub const PORTAL_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>🦀 Power-Crab Setup</title>
  <style>
    :root {
      --bg: #0f172a;
      --card: #1e293b;
      --border: #334155;
      --primary: #f97316;
      --primary-hover: #ea580c;
      --text: #f8fafc;
      --text-muted: #94a3b8;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 100vh;
      padding: 1rem;
    }
    .card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 1rem;
      padding: 2rem;
      width: 100%;
      max-width: 400px;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
    }
    .header { text-align: center; margin-bottom: 1.5rem; }
    .header h1 { font-size: 1.5rem; font-weight: 700; color: var(--primary); }
    .header p { color: var(--text-muted); font-size: 0.875rem; margin-top: 0.25rem; }
    .form-group { margin-bottom: 1.25rem; }
    label { display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 0.5rem; }
    input {
      width: 100%;
      padding: 0.75rem 1rem;
      background: #0f172a;
      border: 1px solid var(--border);
      border-radius: 0.5rem;
      color: var(--text);
      font-size: 1rem;
      outline: none;
      transition: border-color 0.2s;
    }
    input:focus { border-color: var(--primary); }
    button {
      width: 100%;
      padding: 0.75rem;
      background: var(--primary);
      border: none;
      border-radius: 0.5rem;
      color: #fff;
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.2s;
    }
    button:hover { background: var(--primary-hover); }
    .footer { text-align: center; margin-top: 1.5rem; font-size: 0.75rem; color: var(--text-muted); }
  </style>
</head>
<body>
  <div class="card">
    <div class="header">
      <h1>🦀 Power-Crab</h1>
      <p>Wi-Fi Provisioning Setup</p>
    </div>
    <form action="/save" method="POST">
      <div class="form-group">
        <label for="ssid">Wi-Fi Network Name (SSID)</label>
        <input type="text" id="ssid" name="ssid" required maxlength="32" placeholder="e.g. MyHomeWifi" autocomplete="off" autocorrect="off" autocapitalize="none">
      </div>
      <div class="form-group">
        <label for="password">Wi-Fi Password</label>
        <input type="password" id="password" name="password" maxlength="64" placeholder="Enter password (if any)">
      </div>
      <button type="submit">💾 Save &amp; Connect</button>
    </form>
    <div class="footer">
      ESP32 Bare-Metal &bull; Power-Crab
    </div>
  </div>
</body>
</html>"#;

/// Response page shown after submitting credentials
pub const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>🦀 Saving...</title>
  <style>
    body {
      background: #0f172a;
      color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 100vh;
      margin: 0;
      padding: 1rem;
      text-align: center;
    }
    .card {
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 1rem;
      padding: 2.5rem 2rem;
      max-width: 400px;
    }
    h1 { color: #22c55e; margin-bottom: 0.75rem; font-size: 1.5rem; }
    p { color: #94a3b8; line-height: 1.5; font-size: 0.95rem; }
  </style>
</head>
<body>
  <div class="card">
    <h1>✅ Credentials Saved!</h1>
    <p>Power-Crab is now connecting to your Wi-Fi network.<br><br>You may now disconnect from this setup hotspot.</p>
  </div>
</body>
</html>"#;

/// Simple URL decoder in `no_std` for application/x-www-form-urlencoded strings
pub fn url_decode<const N: usize>(input: &str) -> Result<String<N>, ()> {
    let mut out: String<N> = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ').map_err(|_| ())?;
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h1 = (bytes[i + 1] as char).to_digit(16);
                let h2 = (bytes[i + 2] as char).to_digit(16);
                match (h1, h2) {
                    (Some(high), Some(low)) => {
                        let ch = ((high << 4) | low) as u8 as char;
                        out.push(ch).map_err(|_| ())?;
                        i += 3;
                    }
                    _ => {
                        out.push('%').map_err(|_| ())?;
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b as char).map_err(|_| ())?;
                i += 1;
            }
        }
    }

    Ok(out)
}

/// Parse URL-encoded form data (`ssid=...&password=...`)
pub fn parse_form_credentials(body: &str) -> Option<WifiCredentials> {
    let mut ssid_raw: Option<&str> = None;
    let mut pass_raw: Option<&str> = None;

    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?.trim();
        let val = parts.next().unwrap_or("").trim();

        if key == "ssid" {
            ssid_raw = Some(val);
        } else if key == "password" {
            pass_raw = Some(val);
        }
    }

    let ssid_decoded: String<MAX_SSID_LEN> = url_decode(ssid_raw?).ok()?;
    let pass_decoded: String<MAX_PASS_LEN> = url_decode(pass_raw.unwrap_or("")).ok()?;

    if ssid_decoded.is_empty() {
        return None;
    }

    WifiCredentials::new(&ssid_decoded, &pass_decoded).ok()
}
