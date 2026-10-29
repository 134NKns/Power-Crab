//! Wi-Fi Credentials & Target PC Configuration Data Model

use heapless::String;

pub const MAX_SSID_LEN: usize = 32;
pub const MAX_PASS_LEN: usize = 64;

/// Errors that can occur during storage operations
#[derive(Debug)]
pub enum StorageError {
    FlashReadError,
    FlashWriteError,
    FlashEraseError,
    BufferTooSmall,
    CorruptedData,
    InvalidUtf8,
}

/// Wi-Fi Credentials & Target PC container
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiCredentials {
    pub ssid: String<MAX_SSID_LEN>,
    pub password: String<MAX_PASS_LEN>,
    pub target_mac: Option<[u8; 6]>,
}

impl WifiCredentials {
    /// Create new credentials with optional target MAC
    pub fn with_target_mac(
        ssid: &str,
        password: &str,
        target_mac: Option<[u8; 6]>,
    ) -> Result<Self, StorageError> {
        let mut s = String::new();
        s.push_str(ssid).map_err(|_| StorageError::BufferTooSmall)?;

        let mut p = String::new();
        p.push_str(password).map_err(|_| StorageError::BufferTooSmall)?;

        Ok(Self {
            ssid: s,
            password: p,
            target_mac,
        })
    }

    /// Create new credentials without target MAC
    pub fn new(ssid: &str, password: &str) -> Result<Self, StorageError> {
        Self::with_target_mac(ssid, password, None)
    }
}
