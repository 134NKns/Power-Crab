//! Station Mode LAN Web Dashboard & REST API
//!
//! Provides the local web dashboard, Wake-on-LAN trigger,
//! and password-protected Wi-Fi/MAC configuration management.

extern crate alloc;
use alloc::format;
use alloc::string::String as AllocString;
use heapless::String;

use crate::wol::format_mac_address;

pub const ADMIN_USER: &str = "admin";
pub const ADMIN_PASSWORD: &str = "pwd_1234";

/// Parsed HTTP requests received on the Station web server
#[derive(Debug, PartialEq, Eq)]
pub enum StationHttpRequest {
    GetDashboard,
    Wake,
    Favicon,
    UpdateMac {
        mac_str: String<18>,
    },
    ChangeWifi {
        ssid: String<32>,
        password: String<64>,
        admin_pwd: String<32>,
    },
    ResetWifi {
        admin_pwd: String<32>,
    },
    Other,
}

/// Generates the HTML for the Station LAN Web Dashboard
pub fn generate_dashboard_html(
    ssid: &str,
    ip: &str,
    gateway: &str,
    target_mac: Option<[u8; 6]>,
) -> AllocString {
    let mac_display = match target_mac {
        Some(mac) => AllocString::from(format_mac_address(mac).as_str()),
        None => AllocString::from("Not Configured"),
    };

    let mac_val = match target_mac {
        Some(mac) => AllocString::from(format_mac_address(mac).as_str()),
        None => AllocString::new(),
    };

    format!(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Power-Crab</title><style>
:root{{--b:#0f111a;--c:#1a1c29;--t:#fff;--s:#8f9bb3;--a:#2ed573}}
*{{box-sizing:border-box;margin:0;padding:0;font-family:-apple-system,sans-serif}}
body{{background:var(--b);color:var(--t);display:flex;justify-content:center;padding:1.5rem}}
.card{{width:100%;max-width:390px;background:var(--c);border:1px solid #2a2e45;border-radius:14px;padding:1.5rem}}
h1{{font-size:1.35rem;text-align:center;margin-bottom:.3rem}}
.badge{{display:table;margin:0 auto 1rem;padding:.2rem .6rem;border-radius:10px;font-size:.75rem;background:#2ed57322;color:var(--a)}}
.box{{background:#ffffff08;border-radius:10px;padding:.8rem;margin-bottom:1rem;font-size:.85rem;line-height:1.6}}
.r{{display:flex;justify-content:space-between}}.l{{color:var(--s)}}.v{{font-weight:600}}
.btn{{width:100%;padding:.85rem;border-radius:10px;border:none;font-size:1rem;font-weight:700;cursor:pointer;margin-bottom:1rem}}
.btn-w{{background:linear-gradient(135deg,#2ed573,#1e90ff);color:#fff}}
.btn-s{{background:#2f3542;color:#fff;font-size:.85rem;padding:.7rem}}
.btn-d{{background:#ff475722;color:#ff4757;border:1px solid #ff475744;font-size:.85rem;padding:.7rem}}
input{{width:100%;padding:.65rem;border-radius:8px;border:1px solid #2a2e45;background:#12141f;color:#fff;margin:.3rem 0 .7rem;font-size:.9rem}}
details{{background:#ffffff05;border-radius:8px;padding:.7rem;margin-top:.8rem;font-size:.85rem}}
summary{{cursor:pointer;font-weight:600;color:var(--s)}}
.msg{{display:none;padding:.6rem;border-radius:8px;margin-bottom:1rem;font-size:.85rem;text-align:center}}
</style></head><body><div class="card">
<h1>🦀 Power-Crab</h1><div class="badge">● Online on LAN</div>
<div id="m" class="msg"></div>
<div class="box">
<div class="r"><span class="l">SSID:</span><span class="v">{ssid}</span></div>
<div class="r"><span class="l">IP:</span><span class="v">{ip}</span></div>
<div class="r"><span class="l">Gateway:</span><span class="v">{gateway}</span></div>
<div class="r"><span class="l">Target MAC:</span><span class="v" id="mdisp">{mac_display}</span></div>
</div>
<button class="btn btn-w" onclick="wake()">⚡ WAKE UP PC</button>
<label style="font-size:.8rem;color:var(--s)">Target PC MAC:</label>
<input id="mac" placeholder="e.g. 58:D3:12:AD:1C:E8" value="{mac_val}">
<button class="btn btn-s" onclick="saveMac()">💾 Save Target MAC</button>
<details><summary>⚙️ Change Wi-Fi / Reset (Admin)</summary>
<div style="margin-top:.8rem">
<input type="password" id="ap" placeholder="Admin Password (pwd_1234)">
<input id="ns" placeholder="New SSID">
<input type="password" id="np" placeholder="New Password">
<button class="btn btn-s" onclick="chWifi()">Apply New Wi-Fi</button>
<button class="btn btn-d" onclick="rst()">🔄 Factory Reset Wi-Fi</button>
</div></details></div>
<script>
function toast(s,ok){{const e=document.getElementById('m');e.style.display='block';e.style.background=ok?'#2ed57333':'#ff475733';e.style.color=ok?'#7bed9f':'#ff6b81';e.innerText=s;setTimeout(()=>e.style.display='none',4000)}}
function wake(){{fetch('/api/wake',{{method:'POST'}}).then(r=>r.json()).then(d=>toast(d.status=='ok'?'⚡ Sent to '+d.mac:d.message,d.status=='ok')).catch(e=>toast(e,0))}}
function saveMac(){{fetch('/api/config',{{method:'POST',body:'mac='+encodeURIComponent(document.getElementById('mac').value)}}).then(r=>r.json()).then(d=>{{if(d.status=='ok'){{toast('✅ Saved: '+d.mac,1);document.getElementById('mdisp').innerText=d.mac}}else toast(d.message,0)}})}}
function chWifi(){{const a=document.getElementById('ap').value,s=document.getElementById('ns').value,p=document.getElementById('np').value;fetch('/api/wifi',{{method:'POST',body:'admin='+encodeURIComponent(a)+'&ssid='+encodeURIComponent(s)+'&password='+encodeURIComponent(p)}}).then(r=>r.json()).then(d=>toast(d.message,d.status=='ok'))}}
function rst(){{const a=document.getElementById('ap').value;if(!confirm('Reset Wi-Fi?'))return;fetch('/api/reset',{{method:'POST',body:'admin='+encodeURIComponent(a)}}).then(r=>r.json()).then(d=>toast(d.message,d.status=='ok'))}}
</script></body></html>"#,
        ssid = ssid,
        ip = ip,
        gateway = gateway,
        mac_display = mac_display.as_str(),
        mac_val = mac_val.as_str()
    )
}

/// Parses an HTTP request received in Station mode
pub fn parse_station_request(data: &[u8]) -> StationHttpRequest {
    let req_str = match core::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return StationHttpRequest::Other,
    };

    let first_line = req_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("");

    // Strip query string if any
    let path = raw_path.split('?').next().unwrap_or("");

    if (method == "GET" || method == "HEAD") && (path == "" || path == "/" || path == "/index.html") {
        return StationHttpRequest::GetDashboard;
    }

    if (method == "POST" && path == "/api/wake") || ((method == "GET" || method == "HEAD") && path == "/wake") {
        return StationHttpRequest::Wake;
    }

    if path == "/favicon.ico" {
        return StationHttpRequest::Favicon;
    }

    // Extract body if POST
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");

    if method == "POST" && path == "/api/config" {
        if let Some(mac_str) = extract_param(body, "mac") {
            return StationHttpRequest::UpdateMac { mac_str };
        }
    }

    if method == "POST" && path == "/api/wifi" {
        let admin_pwd = extract_param(body, "admin").unwrap_or_default();
        let ssid = extract_param(body, "ssid").unwrap_or_default();
        let password = extract_param(body, "password").unwrap_or_default();
        return StationHttpRequest::ChangeWifi {
            ssid,
            password,
            admin_pwd,
        };
    }

    if method == "POST" && path == "/api/reset" {
        let admin_pwd = extract_param(body, "admin").unwrap_or_default();
        return StationHttpRequest::ResetWifi { admin_pwd };
    }

    StationHttpRequest::Other
}

fn extract_param<const N: usize>(body: &str, key: &str) -> Option<String<N>> {
    for pair in body.split('&') {
        let mut kv = pair.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == key {
                let mut out = String::new();
                let decoded = url_decode(v);
                let _ = out.push_str(&decoded);
                return Some(out);
            }
        }
    }
    None
}

fn url_decode(s: &str) -> AllocString {
    let mut out = AllocString::new();
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => out.push(' '),
            b'%' => {
                let h1 = bytes.next().unwrap_or(b'0');
                let h2 = bytes.next().unwrap_or(b'0');
                let hex_val = hex_nibble(h1) * 16 + hex_nibble(h2);
                out.push(hex_val as char);
            }
            _ => out.push(b as char),
        }
    }
    out
}

fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Formats a complete HTTP response (HTML or JSON)
pub fn format_http_response(status_code: u16, content_type: &str, body: &str, out: &mut [u8]) -> usize {
    let status_line = match status_code {
        200 => "200 OK",
        204 => "204 No Content",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };

    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status_line,
        content_type,
        body.len()
    );

    let header_bytes = header.as_bytes();
    let body_bytes = body.as_bytes();
    let total = header_bytes.len() + body_bytes.len();

    if total <= out.len() {
        out[..header_bytes.len()].copy_from_slice(header_bytes);
        out[header_bytes.len()..total].copy_from_slice(body_bytes);
        total
    } else {
        0
    }
}
