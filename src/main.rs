#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_wifi::wifi::{AccessPointConfiguration, AuthMethod, Configuration, WifiApDevice};
use power_crab::storage::Storage;

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
    match storage.load_wifi() {
        Ok(Some(creds)) => {
            println!("💾 Found saved Wi-Fi in Flash: {}", creds.ssid);
        }
        Ok(None) => {
            println!("⚠️ No Wi-Fi saved in Flash -> Starting SoftAP");
        }
        Err(e) => {
            println!("❌ Storage error: {:?}", e);
        }
    }

    println!("📡 Initializing Wi-Fi radio...");
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    println!("📡 Starting SoftAP: 'POWER-CRAB'...");
    let (_wifi_device, mut controller) =
        esp_wifi::wifi::new_with_mode(&init, peripherals.WIFI, WifiApDevice).unwrap();

    let ap_config = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "POWER-CRAB".try_into().unwrap(),
        ssid_hidden: false,
        channel: 1,
        auth_method: AuthMethod::None,
        password: "".try_into().unwrap(),
        max_connections: 4,
        ..Default::default()
    });

    controller.set_configuration(&ap_config).unwrap();
    controller.start().unwrap();

    println!("✅ SoftAP 'POWER-CRAB' is broadcasting! (Open network, channel 1)");

    let mut counter = 0;
    loop {
        println!("🦀 Power-Crab running... #{}", counter);
        counter += 1;
        delay.delay_millis(3000);
    }
}
