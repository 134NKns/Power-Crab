//! Time Utilities for smoltcp Network Stack

use smoltcp::time::Instant;

/// Returns current system time as a `smoltcp::time::Instant`.
pub fn now_instant() -> Instant {
    Instant::from_micros(
        esp_hal::time::now().duration_since_epoch().to_micros() as i64,
    )
}
