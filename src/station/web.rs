//! Station Mode LAN Web Dashboard & REST API
//!
//! Provides the local web dashboard, Wake-on-LAN trigger,
//! LAN device discovery, live PC status, and password-protected Wi-Fi/MAC configuration.

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
    Preflight,
    GetDevices,
    GetStatus,
    ScanLan,
    ScanWifi,
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

    format!(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>🦀 Power-Crab</title><style>
:root{{--b:#0b0d14;--c:#141724;--cb:#1e2235;--t:#f1f2f6;--s:#8f9bb3;--a:#2ed573;--w:#ffa502;--d:#ff4757;--p:#3742fa}}
*{{box-sizing:border-box;margin:0;padding:0;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif}}
body{{background:var(--b);color:var(--t);display:flex;justify-content:center;padding:1.2rem;min-height:100vh}}
.card{{width:100%;max-width:420px;background:var(--c);border:1px solid #252a3d;border-radius:16px;padding:1.5rem;box-shadow:0 8px 30px rgba(0,0,0,0.5)}}
h1{{font-size:1.45rem;text-align:center;margin-bottom:.2rem}}
.sub{{text-align:center;color:var(--s);font-size:.78rem;margin-bottom:.9rem}}
.badges{{display:flex;justify-content:center;gap:.5rem;margin-bottom:1.1rem;flex-wrap:wrap}}
.badge{{padding:.25rem .65rem;border-radius:12px;font-size:.75rem;font-weight:600}}
.b-lan{{background:#2ed5731f;color:var(--a);border:1px solid #2ed5733d}}
.b-on{{background:#2ed57322;color:var(--a);border:1px solid #2ed57355}}
.b-off{{background:#ff475722;color:var(--d);border:1px solid #ff475755}}
.b-waking{{background:#ffa50222;color:var(--w);border:1px solid #ffa50255;animation:pulse 1.2s infinite}}
@keyframes pulse{{0%,100%{{opacity:1}}50%{{opacity:.5}}}}
.box{{background:#ffffff06;border:1px solid #252a3d;border-radius:12px;padding:.85rem;margin-bottom:1rem;font-size:.85rem;line-height:1.65}}
.r{{display:flex;justify-content:space-between;align-items:center}}.l{{color:var(--s)}}.v{{font-weight:600;font-family:monospace}}
.btn{{width:100%;padding:.85rem;border-radius:10px;border:none;font-size:.95rem;font-weight:700;cursor:pointer;transition:.2s;display:flex;align-items:center;justify-content:center;gap:.5rem}}
.btn:hover{{opacity:.92}}
.btn-w{{background:linear-gradient(135deg,#2ed573,#1e90ff);color:#fff;font-size:1.05rem;padding:.95rem;box-shadow:0 4px 15px #2ed57333;margin-bottom:.8rem}}
.btn-s{{background:var(--cb);color:#fff;border:1px solid #2d334d;font-size:.82rem;padding:.65rem;margin-bottom:.5rem}}
.btn-pick{{background:#2ed57322;color:var(--a);border:1px solid #2ed57344;padding:.35rem .65rem;border-radius:6px;font-size:.75rem;cursor:pointer;font-weight:600}}
.btn-pick:hover{{background:var(--a);color:#000}}
.btn-d{{background:#ff475722;color:var(--d);border:1px solid #ff475744;font-size:.82rem;padding:.65rem}}
input,select{{width:100%;padding:.65rem .75rem;border-radius:8px;border:1px solid #2d334d;background:#0d0f17;color:#fff;margin:.3rem 0 .7rem;font-size:.88rem;outline:none}}
input:focus,select:focus{{border-color:#3742fa}}
.dev-list{{max-height:180px;overflow-y:auto;margin:.6rem 0;display:flex;flex-direction:column;gap:.4rem}}
.dev-item{{background:#ffffff05;border:1px solid #252a3d;border-radius:8px;padding:.5rem .7rem;display:flex;justify-content:space-between;align-items:center;font-size:.8rem}}
.dev-ip{{font-weight:700;color:#fff}}.dev-mac{{font-family:monospace;color:var(--s);font-size:.75rem}}
details{{background:#ffffff04;border:1px solid #252a3d;border-radius:10px;padding:.7rem;margin-top:.8rem;font-size:.85rem}}
summary{{cursor:pointer;font-weight:600;color:var(--s);user-select:none}}
.msg{{display:none;padding:.65rem;border-radius:8px;margin-bottom:1rem;font-size:.85rem;text-align:center;font-weight:600}}
.tag{{font-size:.7rem;color:var(--s);margin-bottom:.2rem;display:block}}
</style></head><body><div class="card">
<h1>🦀 Power-Crab</h1>
<div class="sub">Local Wake-on-LAN Appliance (Internal LAN)</div>
<div class="badges">
<span class="badge b-lan">● LAN: {ip}</span>
<span id="st" class="badge b-off">Checking PC...</span>
</div>
<div id="m" class="msg"></div>
<button id="bw" class="btn btn-w" onclick="wake()">⚡ WAKE UP PC</button>
<div class="box">
<div class="r"><span class="l">Target PC MAC:</span><span class="v" id="mdisp">{mac_display}</span></div>
<div class="r"><span class="l">Connected Wi-Fi:</span><span class="v">{ssid}</span></div>
<div class="r"><span class="l">Gateway:</span><span class="v">{gateway}</span></div>
</div>
<details open><summary>🔍 Find PC on LAN (ไม่ต้องพิมพ์ MAC)</summary>
<div style="margin-top:.7rem">
<button class="btn btn-s" onclick="scanLan()">🔍 Scan Active Devices in Home</button>
<div id="dl" class="dev-list"><div style="text-align:center;color:var(--s);font-size:.8rem;padding:.8rem">กดปุ่มสแกนเพื่อค้นหาเครื่องในวง LAN</div></div>
<span class="tag">หรือระบุ MAC ด้วยตนเอง:</span>
<input id="mac" placeholder="XX:XX:XX:XX:XX:XX" value="{mac_val}">
<button class="btn btn-s" onclick="saveMac()">💾 Save Target MAC</button>
</div></details>
<details><summary>📲 Local Shortcut &amp; Webhook URL</summary>
<div style="margin-top:.7rem">
<span class="tag">Direct Wake URL (สำหรับ iOS Shortcuts / Siri / Stream Deck):</span>
<input id="hu" readonly value="http://{ip}/wake">
<button class="btn btn-s" onclick="copyHook()">📋 Copy Shortcut URL</button>
<div style="font-size:.75rem;color:var(--s);line-height:1.4">ทำงานเฉพาะภายในบ้านเท่านั้น ปลอดภัย 100% ไม่ต้องใช้ Domain หรือเน็ตนอกบ้าน</div>
</div></details>
<details><summary>⚙️ Change Wi-Fi / Factory Reset (Admin)</summary>
<div style="margin-top:.7rem">
<input type="password" id="ap" placeholder="Admin Password">
<button class="btn btn-s" onclick="scanWifi()">📶 Scan Nearby Wi-Fi</button>
<select id="ws" onchange="pickWifi(this)"><option value="">-- เลือก Wi-Fi (หรือกด Scan) --</option></select>
<input id="ns" placeholder="Wi-Fi SSID">
<input type="password" id="np" placeholder="Wi-Fi Password">
<button class="btn btn-s" onclick="chWifi()">Apply New Wi-Fi</button>
<button class="btn btn-d" onclick="rst()">🔄 Factory Reset Wi-Fi</button>
</div></details>
</div>
<script>
function toast(s,ok){{const e=document.getElementById('m');e.style.display='block';e.style.background=ok?'#2ed57329':'#ff475729';e.style.color=ok?'#2ed573':'#ff4757';e.style.border='1px solid '+(ok?'#2ed57355':'#ff475755');e.innerText=s;setTimeout(()=>e.style.display='none',4500)}}
function checkStatus(){{fetch('/api/status').then(r=>r.json()).then(d=>{{const s=document.getElementById('st');if(d.online){{s.className='badge b-on';s.innerText='🟢 PC Online'}}else{{s.className='badge b-off';s.innerText='🔴 PC Offline'}}if(d.online&&window.waking){{window.waking=false;toast('🎉 Target PC is now ONLINE!',1)}}}}).catch(()=>{{}})}}
function wake(){{window.waking=true;const s=document.getElementById('st');s.className='badge b-waking';s.innerText='🟡 Waking up...';fetch('/api/wake',{{method:'POST'}}).then(r=>r.json()).then(d=>{{toast(d.status=='ok'?'⚡ Magic packet broadcasted to '+d.mac:d.message,d.status=='ok');for(let i=1;i<=8;i++)setTimeout(checkStatus,i*2500)}}).catch(e=>toast(e,0))}}
function saveMac(){{const v=document.getElementById('mac').value.trim();fetch('/api/config',{{method:'POST',body:'mac='+encodeURIComponent(v)}}).then(r=>r.json()).then(d=>{{if(d.status=='ok'){{toast('✅ Saved: '+d.mac,1);document.getElementById('mdisp').innerText=d.mac;checkStatus()}}else toast(d.message,0)}})}}
function pickMac(m){{document.getElementById('mac').value=m;saveMac()}}
function scanLan(){{const dl=document.getElementById('dl');dl.innerHTML='<div style="text-align:center;color:var(--s);font-size:.8rem;padding:.5rem">🔍 Scanning LAN...</div>';fetch('/api/scan-lan',{{method:'POST'}}).catch(()=>{{}});setTimeout(()=>{{fetch('/api/devices').then(r=>r.json()).then(list=>{{if(!list||!list.length){{dl.innerHTML='<div style="text-align:center;color:var(--s);font-size:.8rem;padding:.5rem">No devices detected yet</div>';return}}let h='';list.forEach(d=>{{h+='<div class="dev-item"><div><div class="dev-ip">'+d.ip+'</div><div class="dev-mac">'+d.mac+' ('+d.ago+'s ago)</div></div><button class="btn-pick" onclick="pickMac(\''+d.mac+'\')">👉 Use Target</button></div>'}});dl.innerHTML=h}}).catch(e=>{{dl.innerHTML='<div style="color:var(--d);font-size:.8rem">'+e+'</div>'}})}},1200)}}
function scanWifi(){{fetch('/api/wifi/scan').then(r=>r.json()).then(list=>{{const sel=document.getElementById('ws');sel.innerHTML='<option value="">-- เลือก Wi-Fi --</option>';list.forEach(w=>{{const opt=document.createElement('option');opt.value=w.ssid;opt.text='📶 '+w.ssid+' ('+w.rssi+' dBm)';sel.appendChild(opt)}});toast('✅ Found '+list.length+' Wi-Fi networks',1)}}).catch(e=>toast(e,0))}}
function pickWifi(s){{if(s.value)document.getElementById('ns').value=s.value}}
function copyHook(){{const u=document.getElementById('hu');u.select();navigator.clipboard.writeText(u.value).then(()=>toast('📋 Copied Shortcut URL!',1))}}
function chWifi(){{const a=document.getElementById('ap').value,s=document.getElementById('ns').value,p=document.getElementById('np').value;fetch('/api/wifi',{{method:'POST',body:'admin='+encodeURIComponent(a)+'&ssid='+encodeURIComponent(s)+'&password='+encodeURIComponent(p)}}).then(r=>r.json()).then(d=>toast(d.message,d.status=='ok'))}}
function rst(){{const a=document.getElementById('ap').value;if(!confirm('Reset Wi-Fi?'))return;fetch('/api/reset',{{method:'POST',body:'admin='+encodeURIComponent(a)}}).then(r=>r.json()).then(d=>toast(d.message,d.status=='ok'))}}
checkStatus();setInterval(checkStatus,10000);
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

    // Handle CORS preflight for ALL paths (Brave/Chrome private network access fix)
    if method == "OPTIONS" {
        return StationHttpRequest::Preflight;
    }

    if (method == "GET" || method == "HEAD") && (path == "" || path == "/" || path == "/index.html") {
        return StationHttpRequest::GetDashboard;
    }

    if (method == "POST" && path == "/api/wake") || ((method == "GET" || method == "HEAD") && path == "/wake") {
        return StationHttpRequest::Wake;
    }

    if (method == "GET" || method == "HEAD") && path == "/api/devices" {
        return StationHttpRequest::GetDevices;
    }

    if (method == "GET" || method == "HEAD") && path == "/api/status" {
        return StationHttpRequest::GetStatus;
    }

    if (method == "POST" || method == "GET") && path == "/api/scan-lan" {
        return StationHttpRequest::ScanLan;
    }

    if (method == "GET" || method == "POST") && path == "/api/wifi/scan" {
        return StationHttpRequest::ScanWifi;
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

    // Access-Control-Allow-Private-Network fixes Brave/Chrome private network access blocking
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\n\r\n",
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
