//! Over-the-air updates, fetched by the board: `firmware/tools/ota.py` serves the image on the Mac and publishes its
//! address on MQTT (`sauter-marapi/ota/set`); the board connects to it, reads the OTA password on one line, then
//! the app image (`espflash save-image`) until the server closes. The image goes to the free OTA slot, which boots
//! next. No port is open on the board. While the flash is written, interrupts wait: a few polls are missed, which
//! the power board tolerates.
use core::net::SocketAddrV4;

use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use esp_bootloader_esp_idf::ota::OtaImageState;
use esp_bootloader_esp_idf::ota_updater::OtaUpdater;
use esp_bootloader_esp_idf::partitions::{self, FlashRegion, PARTITION_TABLE_MAX_LEN};
use esp_storage::FlashStorage;
use log::{info, warn};

use crate::secrets::OTA_PASSWORD;

static REQUEST: Signal<CriticalSectionRawMutex, SocketAddrV4> = Signal::new();
const SECTOR: usize = 4096;
/// First byte of an ESP app image.
const IMAGE_MAGIC: u8 = 0xE9;

#[derive(Debug)]
#[expect(dead_code, reason = "the fields are read through Debug, in the logs")]
enum OtaError {
    Network,
    Password,
    NotAnImage,
    Flash(partitions::Error),
}

impl From<partitions::Error> for OtaError {
    fn from(error: partitions::Error) -> Self {
        Self::Flash(error)
    }
}

/// Marks the running image valid (only matters with a rollback bootloader) and says which slot runs.
pub fn confirm_running_image(flash: &mut FlashStorage<'static>) {
    let mut buffer = [0u8; PARTITION_TABLE_MAX_LEN];
    let Ok(mut ota) = OtaUpdater::new(flash, &mut buffer) else { return };
    if let Ok(OtaImageState::New | OtaImageState::PendingVerify) = ota.current_ota_state() {
        let _ = ota.set_current_ota_state(OtaImageState::Valid);
    }
    info!("running from {:?}", ota.selected_partition());
}

/// Asks the OTA task to fetch an image from this address (an MQTT command).
pub fn request(source: SocketAddrV4) {
    REQUEST.signal(source);
}

#[embassy_executor::task]
pub async fn serve(stack: Stack<'static>, mut flash: FlashStorage<'static>) {
    let (mut rx, mut tx) = ([0u8; 2048], [0u8; 64]);
    loop {
        let source = REQUEST.wait().await;
        info!("OTA: fetching the image from {source}");
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(30)));
        let result = match socket.connect(source).await {
            Ok(()) => update(&mut socket, &mut flash).await,
            Err(_) => Err(OtaError::Network),
        };
        let reply: &[u8] = if result.is_ok() { b"OK\n" } else { b"FAILED\n" };
        let _ = socket.write(reply).await;
        let _ = socket.flush().await;
        socket.close();
        match result {
            Ok(bytes) => {
                info!("OTA: {bytes} bytes written, rebooting into the new image");
                embassy_time::Timer::after_secs(1).await;
                esp_hal::system::software_reset();
            }
            Err(error) => warn!("OTA failed: {error:?}"),
        }
    }
}

async fn update(socket: &mut TcpSocket<'_>, flash: &mut FlashStorage<'static>) -> Result<u32, OtaError> {
    check_password(socket).await?;
    let mut buffer = [0u8; PARTITION_TABLE_MAX_LEN];
    let mut ota = OtaUpdater::new(flash, &mut buffer)?;
    let written = {
        let (mut region, slot) = ota.next_partition()?;
        info!("OTA: writing {slot:?}");
        copy(socket, &mut region).await?
    };
    ota.activate_next_partition()?;
    ota.set_current_ota_state(OtaImageState::New)?;
    Ok(written)
}

async fn check_password(socket: &mut TcpSocket<'_>) -> Result<(), OtaError> {
    let mut line = [0u8; 64];
    let mut len = 0;
    loop {
        let mut byte = [0u8; 1];
        if socket.read(&mut byte).await.map_err(|_| OtaError::Network)? == 0 || len == line.len() {
            return Err(OtaError::Password);
        }
        if byte[0] == b'\n' {
            break;
        }
        line[len] = byte[0];
        len += 1;
    }
    if &line[..len] == OTA_PASSWORD.as_bytes() { Ok(()) } else { Err(OtaError::Password) }
}

/// The image, sector by sector, until the sender closes.
async fn copy(socket: &mut TcpSocket<'_>, region: &mut FlashRegion<'_, '_>) -> Result<u32, OtaError> {
    let mut sector = [0u8; SECTOR];
    let mut offset = 0u32;
    loop {
        let len = fill(socket, &mut sector).await?;
        if len == 0 {
            break;
        }
        if offset == 0 && sector[0] != IMAGE_MAGIC {
            return Err(OtaError::NotAnImage);
        }
        region.erase(offset, offset + SECTOR as u32)?;
        region.write(offset, &sector[..len])?;
        offset += len as u32;
        if len < SECTOR {
            break;
        }
    }
    if offset == 0 { Err(OtaError::NotAnImage) } else { Ok(offset) }
}

/// Reads until the buffer is full or the sender closes; returns the length read.
async fn fill(socket: &mut TcpSocket<'_>, buffer: &mut [u8]) -> Result<usize, OtaError> {
    let mut len = 0;
    while len < buffer.len() {
        match socket.read(&mut buffer[len..]).await.map_err(|_| OtaError::Network)? {
            0 => break,
            read => len += read,
        }
    }
    Ok(len)
}
