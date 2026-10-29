//! Web Portal Module
//!
//! Provides the Captive Portal HTML UI and HTTP request/form handling.

extern crate alloc;
use alloc::format;
use alloc::string::String as AllocString;
use crate::storage::{WifiCredentials, MAX_PASS_LEN, MAX_SSID_LEN};
use heapless::String;

/// Generate the dynamic setup page with detected Wi-Fi networks in a dropdown.
pub fn generate_portal_html(scanned_ssids: &[String<32>]) -> AllocString {
    let mut options = AllocString::new();
    for ssid in scanned_ssids {
        if !ssid.is_empty() {
            options.push_str(&format!(
                r#"<option value="{ssid}">📶 {ssid}</option>"#,
                ssid = ssid
            ));
        }
    }

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>🦀 Power-Crab Setup</title>
  <style>
    :root {{
      --bg: #0f172a;
      --card: #1e293b;
      --border: #334155;
      --primary: #f97316;
      --primary-hover: #ea580c;
      --text: #f8fafc;
      --text-muted: #94a3b8;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 100vh;
      padding: 1rem;
    }}
    .card {{
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 1rem;
      padding: 2rem;
      width: 100%;
      max-width: 400px;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
    }}
    .header {{ text-align: center; margin-bottom: 1.5rem; }}
    .header h1 {{ font-size: 1.5rem; font-weight: 700; color: var(--primary); }}
    .header p {{ color: var(--text-muted); font-size: 0.875rem; margin-top: 0.25rem; }}
    .form-group {{ margin-bottom: 1.25rem; }}
    label {{ display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 0.5rem; }}
    select, input {{
      width: 100%;
      padding: 0.75rem 1rem;
      background: #0f172a;
      border: 1px solid var(--border);
      border-radius: 0.5rem;
      color: var(--text);
      font-size: 1rem;
      outline: none;
      transition: border-color 0.2s;
    }}
    select:focus, input:focus {{ border-color: var(--primary); }}
    select option {{ background: #1e293b; color: #f8fafc; }}
    button {{
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
      margin-top: 0.5rem;
    }}
    button:hover {{ background: var(--primary-hover); }}
    .footer {{ text-align: center; margin-top: 1.5rem; font-size: 0.75rem; color: var(--text-muted); }}
  </style>
</head>
<body>
  <div class="card">
    <div class="header">
      <h1>🦀 Power-Crab</h1>
      <p>Select your home Wi-Fi</p>
    </div>
    <form action="/save" method="POST">
      <div class="form-group">
        <label for="ssid">Nearby Wi-Fi Networks</label>
        <select id="ssid" name="ssid" required onchange="handleSsidChange(this)">
          <option value="" disabled selected>-- Choose Wi-Fi --</option>
          {options}
          <option value="__custom__">✏️ Enter Manually...</option>
        </select>
        <input type="text" id="custom_ssid" name="custom_ssid" placeholder="Type Wi-Fi Name" style="display:none; margin-top: 0.5rem;" maxlength="32" autocomplete="off">
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
  <script>
    function handleSsidChange(sel) {{
      var custom = document.getElementById('custom_ssid');
      if (sel.value === '__custom__') {{
        custom.style.display = 'block';
        custom.required = true;
        custom.focus();
      }} else {{
        custom.style.display = 'none';
        custom.required = false;
      }}
    }}
  </script>
</body>
</html>"#)
}

/// Generate a rich status result page
pub fn generate_result_html(ssid: &str, success: bool, reason: Option<&str>) -> AllocString {
    let (status_icon, status_title, title_color, message, action_btn) = if success {
        (
            "✅",
            "Connected Successfully!",
            "#22c55e",
            format!(
                r#"Power-Crab is now connected to <b>{}</b>.<br><br>Credentials have been saved to Flash memory. You may now close this window."#,
                ssid
            ),
            AllocString::new(),
        )
    } else {
        (
            "❌",
            "Connection Failed!",
            "#ef4444",
            format!(
                r#"Could not connect to <b>{}</b>.<br><br><span style="color: #f87171;">Reason: {}</span>"#,
                ssid,
                reason.unwrap_or("Incorrect password or network out of range")
            ),
            format!(
                r#"<a href="/" style="display:block;margin-top:1.5rem;padding:0.75rem;background:#f97316;color:#fff;border-radius:0.5rem;text-decoration:none;font-weight:600;">🔄 Try Again</a>"#
            ),
        )
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>🦀 Connection Result</title>
  <style>
    body {{
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
    }}
    .card {{
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 1rem;
      padding: 2.5rem 2rem;
      max-width: 420px;
      width: 100%;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
    }}
    .icon {{ font-size: 2.5rem; margin-bottom: 0.5rem; }}
    h1 {{ color: {title_color}; margin-bottom: 0.75rem; font-size: 1.5rem; font-weight: 700; }}
    p {{ color: #94a3b8; line-height: 1.6; font-size: 0.95rem; }}
    b {{ color: #f8fafc; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">{status_icon}</div>
    <h1>{status_title}</h1>
    <p>{message}</p>
    {action_btn}
  </div>
</body>
</html>"#)
}

/// Parsed HTTP request type
#[derive(Debug)]
pub enum HttpRequest {
    GetPortal,
    SubmitCredentials(WifiCredentials),
    Other,
}

/// Parse incoming HTTP request buffer
pub fn parse_http_request(req: &[u8]) -> HttpRequest {
    let req_str = match core::str::from_utf8(req) {
        Ok(s) => s,
        Err(_) => return HttpRequest::Other,
    };

    if req_str.starts_with("POST /save") {
        if let Some(body_start) = req_str.find("\r\n\r\n") {
            let body = &req_str[body_start + 4..];
            if let Some(creds) = parse_form_credentials(body) {
                return HttpRequest::SubmitCredentials(creds);
            }
        }
    }

    HttpRequest::GetPortal
}

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

/// Parse URL-encoded form data (`ssid=...&custom_ssid=...&password=...`)
pub fn parse_form_credentials(body: &str) -> Option<WifiCredentials> {
    let mut ssid_raw: Option<&str> = None;
    let mut custom_ssid_raw: Option<&str> = None;
    let mut pass_raw: Option<&str> = None;

    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?.trim();
        let val = parts.next().unwrap_or("").trim();

        if key == "ssid" {
            ssid_raw = Some(val);
        } else if key == "custom_ssid" {
            custom_ssid_raw = Some(val);
        } else if key == "password" {
            pass_raw = Some(val);
        }
    }

    let mut chosen_ssid = ssid_raw?;
    if chosen_ssid == "__custom__" {
        if let Some(custom) = custom_ssid_raw {
            if !custom.is_empty() {
                chosen_ssid = custom;
            }
        }
    }

    let ssid_decoded: String<MAX_SSID_LEN> = url_decode(chosen_ssid).ok()?;
    let pass_decoded: String<MAX_PASS_LEN> = url_decode(pass_raw.unwrap_or("")).ok()?;

    if ssid_decoded.is_empty() {
        return None;
    }

    WifiCredentials::new(&ssid_decoded, &pass_decoded).ok()
}

fn format_usize(mut val: usize, buf: &mut [u8; 10]) -> &str {
    if val == 0 {
        return "0";
    }
    let mut idx = buf.len();
    while val > 0 && idx > 0 {
        idx -= 1;
        buf[idx] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    unsafe { core::str::from_utf8_unchecked(&buf[idx..]) }
}

/// Formats an HTTP response with status 200 OK, Content-Type: text/html, and body.
pub fn format_http_response(body: &str, out: &mut [u8]) -> usize {
    let header_prefix = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: ";
    let header_suffix = "\r\n\r\n";

    let body_len = body.len();
    let mut len_buf = [0u8; 10];
    let len_str = format_usize(body_len, &mut len_buf);

    let total_len = header_prefix.len() + len_str.len() + header_suffix.len() + body_len;
    if total_len > out.len() {
        return 0;
    }

    let mut cursor = 0;
    out[cursor..cursor + header_prefix.len()].copy_from_slice(header_prefix.as_bytes());
    cursor += header_prefix.len();

    out[cursor..cursor + len_str.len()].copy_from_slice(len_str.as_bytes());
    cursor += len_str.len();

    out[cursor..cursor + header_suffix.len()].copy_from_slice(header_suffix.as_bytes());
    cursor += header_suffix.len();

    out[cursor..cursor + body.len()].copy_from_slice(body.as_bytes());
    cursor += body.len();

    cursor
}
