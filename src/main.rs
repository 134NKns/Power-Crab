#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, main, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use power_crab::{
    portal::run_captive_portal,
    station::run_station_mode,
    storage::Storage,
};

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(72 * 1024);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let delay = Delay::new();

    println!();
    println!("========================================");
    println!("🦀 Hello, Power-Crab (esp-hal bare-metal)");
    println!("========================================");

    let mut storage = Storage::new();
    let saved_wifi = match storage.load_wifi() {
        Ok(Some(creds)) => {
            println!(
                "💾 Found saved Wi-Fi in Flash: '{}' (pass_len: {})",
                creds.ssid,
                creds.password.len()
            );
            Some(creds)
        }
        Ok(None) => {
            println!("⚠️ No Wi-Fi saved in Flash -> Starting Captive Portal");
            None
        }
        Err(e) => {
            println!("❌ Storage error: {:?}", e);
            None
        }
    };

    println!("📡 Initializing Wi-Fi radio...");
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    // Mode 1: If credentials exist in Flash, run in Station (STA) mode
    if let Some(creds) = saved_wifi {
        run_station_mode(&init, peripherals.WIFI, &creds, &delay, &mut storage);
    }

    // Mode 2: No credentials saved -> Run Captive Portal
    println!("📡 Starting Captive Portal: 'POWER-CRAB'...");
    run_captive_portal(&init, peripherals.WIFI, &delay, &mut storage);
}
