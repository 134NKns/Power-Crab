//! Flash Storage Implementation for ESP32
//!
//! Stores Wi-Fi credentials and Target PC MAC address in sector 0x3FF000.

use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_storage::FlashStorage;

use super::creds::{StorageError, WifiCredentials, MAX_PASS_LEN, MAX_SSID_LEN};
use crate::traits::WifiStorage;

pub const WIFI_CONFIG_FLASH_ADDR: u32 = 0x3FF000;
pub const STORAGE_MAGIC: [u8; 4] = [0x50, 0x43, 0x57, 0x46]; // "PCWF"
pub const STORAGE_VERSION_V1: u8 = 1;
pub const STORAGE_VERSION_V2: u8 = 2;

#[repr(C)]
struct StoredPayloadV1 {
    magic: [u8; 4],
    version: u8,
    ssid_len: u8,
    ssid_data: [u8; MAX_SSID_LEN],
    pass_len: u8,
    pass_data: [u8; MAX_PASS_LEN],
    checksum: u8,
}

impl StoredPayloadV1 {
    fn compute_checksum(&self) -> u8 {
        let mut chk: u8 = self.version ^ self.ssid_len ^ self.pass_len;
        for &b in &self.ssid_data {
            chk ^= b;
        }
        for &b in &self.pass_data {
            chk ^= b;
        }
        chk
    }
}

#[repr(C)]
struct StoredPayloadV2 {
    magic: [u8; 4],
    version: u8,
    ssid_len: u8,
    ssid_data: [u8; MAX_SSID_LEN],
    pass_len: u8,
    pass_data: [u8; MAX_PASS_LEN],
    has_mac: u8,
    mac_data: [u8; 6],
    checksum: u8,
}

impl StoredPayloadV2 {
    fn compute_checksum(&self) -> u8 {
        let mut chk: u8 = self.version ^ self.ssid_len ^ self.pass_len ^ self.has_mac;
        for &b in &self.ssid_data {
            chk ^= b;
        }
        for &b in &self.pass_data {
            chk ^= b;
        }
        for &b in &self.mac_data {
            chk ^= b;
        }
        chk
    }
}

/// Hardware flash-backed storage manager
pub struct Storage {
    flash: FlashStorage,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage {
    /// Initialize a new Storage instance
    pub fn new() -> Self {
        Self {
            flash: FlashStorage::new(),
        }
    }

    /// Read saved Wi-Fi credentials and target MAC from Flash.
    pub fn load_wifi(&mut self) -> Result<Option<WifiCredentials>, StorageError> {
        let mut buffer = [0u8; 128];
        self.flash
            .read(WIFI_CONFIG_FLASH_ADDR, &mut buffer)
            .map_err(|_| StorageError::FlashReadError)?;

        // If sector is unwritten/erased, all bytes are 0xFF
        if buffer.iter().all(|&b| b == 0xFF) {
            return Ok(None);
        }

        // Verify magic bytes
        if buffer[0..4] != STORAGE_MAGIC {
            return Ok(None);
        }

        let version = buffer[4];
        if version == STORAGE_VERSION_V2 {
            let payload = unsafe { &*(buffer.as_ptr() as *const StoredPayloadV2) };
            if payload.compute_checksum() != payload.checksum {
                return Err(StorageError::CorruptedData);
            }

            let ssid_len = payload.ssid_len as usize;
            let pass_len = payload.pass_len as usize;
            if ssid_len > MAX_SSID_LEN || pass_len > MAX_PASS_LEN {
                return Err(StorageError::CorruptedData);
            }

            let ssid_str = core::str::from_utf8(&payload.ssid_data[..ssid_len])
                .map_err(|_| StorageError::InvalidUtf8)?;
            let pass_str = core::str::from_utf8(&payload.pass_data[..pass_len])
                .map_err(|_| StorageError::InvalidUtf8)?;

            let target_mac = if payload.has_mac == 1 {
                Some(payload.mac_data)
            } else {
                None
            };

            let creds = WifiCredentials::with_target_mac(ssid_str, pass_str, target_mac)?;
            Ok(Some(creds))
        } else if version == STORAGE_VERSION_V1 {
            let payload = unsafe { &*(buffer.as_ptr() as *const StoredPayloadV1) };
            if payload.compute_checksum() != payload.checksum {
                return Err(StorageError::CorruptedData);
            }

            let ssid_len = payload.ssid_len as usize;
            let pass_len = payload.pass_len as usize;
            if ssid_len > MAX_SSID_LEN || pass_len > MAX_PASS_LEN {
                return Err(StorageError::CorruptedData);
            }

            let ssid_str = core::str::from_utf8(&payload.ssid_data[..ssid_len])
                .map_err(|_| StorageError::InvalidUtf8)?;
            let pass_str = core::str::from_utf8(&payload.pass_data[..pass_len])
                .map_err(|_| StorageError::InvalidUtf8)?;

            let creds = WifiCredentials::with_target_mac(ssid_str, pass_str, None)?;
            Ok(Some(creds))
        } else {
            Ok(None)
        }
    }

    /// Save new Wi-Fi credentials & target MAC to Flash (writes V2 format).
    pub fn save_wifi(&mut self, creds: &WifiCredentials) -> Result<(), StorageError> {
        let mut payload = StoredPayloadV2 {
            magic: STORAGE_MAGIC,
            version: STORAGE_VERSION_V2,
            ssid_len: creds.ssid.len() as u8,
            ssid_data: [0u8; MAX_SSID_LEN],
            pass_len: creds.password.len() as u8,
            pass_data: [0u8; MAX_PASS_LEN],
            has_mac: if creds.target_mac.is_some() { 1 } else { 0 },
            mac_data: creds.target_mac.unwrap_or([0u8; 6]),
            checksum: 0,
        };

        payload.ssid_data[..creds.ssid.len()].copy_from_slice(creds.ssid.as_bytes());
        payload.pass_data[..creds.password.len()].copy_from_slice(creds.password.as_bytes());
        payload.checksum = payload.compute_checksum();

        // ESP32 Flash requires 4KB sector erase before writing
        self.flash
            .erase(WIFI_CONFIG_FLASH_ADDR, WIFI_CONFIG_FLASH_ADDR + 4096)
            .map_err(|_| StorageError::FlashEraseError)?;

        // Write payload to flash
        let payload_bytes = unsafe {
            core::slice::from_raw_parts(
                &payload as *const StoredPayloadV2 as *const u8,
                core::mem::size_of::<StoredPayloadV2>(),
            )
        };

        let mut aligned_buf = [0xFFu8; 128];
        aligned_buf[..payload_bytes.len()].copy_from_slice(payload_bytes);

        self.flash
            .write(WIFI_CONFIG_FLASH_ADDR, &aligned_buf[..128])
            .map_err(|_| StorageError::FlashWriteError)?;

        Ok(())
    }

    /// Update target MAC without changing Wi-Fi SSID/password.
    pub fn save_target_mac(&mut self, mac: Option<[u8; 6]>) -> Result<(), StorageError> {
        if let Some(mut creds) = self.load_wifi()? {
            creds.target_mac = mac;
            self.save_wifi(&creds)
        } else {
            Err(StorageError::CorruptedData)
        }
    }

    /// Erase the Wi-Fi credentials sector (Factory reset Wi-Fi)
    pub fn erase_wifi(&mut self) -> Result<(), StorageError> {
        self.flash
            .erase(WIFI_CONFIG_FLASH_ADDR, WIFI_CONFIG_FLASH_ADDR + 4096)
            .map_err(|_| StorageError::FlashEraseError)?;
        Ok(())
    }
}

impl WifiStorage for Storage {
    fn load_wifi(&mut self) -> Result<Option<WifiCredentials>, StorageError> {
        self.load_wifi()
    }

    fn save_wifi(&mut self, creds: &WifiCredentials) -> Result<(), StorageError> {
        self.save_wifi(creds)
    }

    fn erase_wifi(&mut self) -> Result<(), StorageError> {
        self.erase_wifi()
    }
}
