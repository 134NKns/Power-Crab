//! Flash Storage Implementation for ESP32

use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_storage::FlashStorage;

use super::creds::{StorageError, WifiCredentials, MAX_PASS_LEN, MAX_SSID_LEN};
use crate::traits::WifiStorage;

/// Flash address reserved for Wi-Fi credentials.
/// Sector size on ESP32 is 4096 bytes (0x1000).
/// 0x3FF000 is the final 4KB sector of a 4MB flash chip (0x3FF000 - 0x3FFFFF),
/// completely isolated from bootloader and application binaries.
pub const WIFI_CONFIG_FLASH_ADDR: u32 = 0x3FF000;

/// Magic 4-byte signature to verify valid initialized data ("PCWF" = Power-Crab Wi-Fi)
pub const STORAGE_MAGIC: [u8; 4] = [0x50, 0x43, 0x57, 0x46];

/// Storage format version
pub const STORAGE_VERSION: u8 = 1;

/// Raw representation packed into Flash sector
/// Total size: 4 (magic) + 1 (ver) + 1 (ssid_len) + 32 (ssid) + 1 (pass_len) + 64 (pass) + 1 (checksum) = 104 bytes
#[repr(C)]
struct StoredPayload {
    magic: [u8; 4],
    version: u8,
    ssid_len: u8,
    ssid_data: [u8; MAX_SSID_LEN],
    pass_len: u8,
    pass_data: [u8; MAX_PASS_LEN],
    checksum: u8,
}

impl StoredPayload {
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

    /// Read saved Wi-Fi credentials from Flash.
    pub fn load_wifi(&mut self) -> Result<Option<WifiCredentials>, StorageError> {
        let mut buffer = [0u8; core::mem::size_of::<StoredPayload>()];
        self.flash
            .read(WIFI_CONFIG_FLASH_ADDR, &mut buffer)
            .map_err(|_| StorageError::FlashReadError)?;

        // If sector is unwritten/erased, all bytes are 0xFF
        if buffer.iter().all(|&b| b == 0xFF) {
            return Ok(None);
        }

        // Safe cast to StoredPayload
        let payload = unsafe { &*(buffer.as_ptr() as *const StoredPayload) };

        // Verify magic bytes
        if payload.magic != STORAGE_MAGIC {
            return Ok(None);
        }

        // Verify version
        if payload.version != STORAGE_VERSION {
            return Ok(None);
        }

        // Verify checksum
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

        let creds = WifiCredentials::new(ssid_str, pass_str)?;
        Ok(Some(creds))
    }

    /// Save new Wi-Fi credentials to Flash.
    pub fn save_wifi(&mut self, creds: &WifiCredentials) -> Result<(), StorageError> {
        let mut payload = StoredPayload {
            magic: STORAGE_MAGIC,
            version: STORAGE_VERSION,
            ssid_len: creds.ssid.len() as u8,
            ssid_data: [0u8; MAX_SSID_LEN],
            pass_len: creds.password.len() as u8,
            pass_data: [0u8; MAX_PASS_LEN],
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
                &payload as *const StoredPayload as *const u8,
                core::mem::size_of::<StoredPayload>(),
            )
        };

        // Align write buffer to 4-byte boundary if needed
        let mut aligned_buf = [0xFFu8; 128];
        aligned_buf[..payload_bytes.len()].copy_from_slice(payload_bytes);

        self.flash
            .write(WIFI_CONFIG_FLASH_ADDR, &aligned_buf[..128])
            .map_err(|_| StorageError::FlashWriteError)?;

        Ok(())
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
