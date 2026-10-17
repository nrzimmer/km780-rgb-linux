//! KM780 RGB transport over HID Feature report 7.
//!
//! Mirrors `tools/km780lib.py`, which is the reference -- the names, the
//! offsets and the waits are the same on purpose. The specification is in
//! `docs/09-protocol.md`.
//!
//! MIND buf[3] vs buf[4]: the color channel of 0x09/0x89 goes in **buf[3]**.
//! The effect helpers (0x0B/0x12/0x15) use buf[4]. That is why `build` takes
//! explicit `b3`/`b4`/`b5` and has no parameter called "channel" -- confusing
//! the two has already caused a bug in this project.

use std::thread::sleep;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

pub const VID: u16 = 0x28DA;
pub const PID: u16 = 0x1101;

/// Packet size, including the report ID byte at position 0.
pub const PKT: usize = 0x108; // 264
pub const HDR: usize = 8;
pub const REPORT_ID: u8 = 0x07;

pub const MODE_GET: u8 = 0x81;
pub const MODE_SET: u8 = 0x01;
pub const INFO_GET: u8 = 0x82;
#[allow(dead_code)]
pub const INFO_SET: u8 = 0x02;
pub const LED_GET: u8 = 0x89;
pub const LED_SET: u8 = 0x09;

// Offsets inside the mode payload (0x02/0x82), absolute within the packet.
#[allow(dead_code)]
pub const OFF_STATUS: usize = 3;
pub const OFF_EFFECT: usize = 10;
pub const OFF_BRIGHTNESS: usize = 11;

/// 1=R 2=G 3=B, tested on hardware.
pub const CHANNELS: [u8; 3] = [1, 2, 3];

const WAIT_SIMPLE: Duration = Duration::from_millis(10);
const WAIT_READ: Duration = Duration::from_millis(50);
const WAIT_CHANNEL: Duration = Duration::from_millis(20);
const WAIT_AFTER_COLOR: Duration = Duration::from_millis(100);

pub type Res<T> = Result<T, String>;

/// The three color channels as they come from 0x89: `[r, g, b]`, each with NSLOTS bytes.
pub type Channels = [Vec<u8>; 3];

pub struct Keyboard {
    dev: HidDevice,
    pub description: String,
}

impl Keyboard {
    /// Opens the right HID entry among the several the same VID/PID exposes.
    ///
    /// The command lives in the vendor collection of usage page 0xFF01,
    /// interface 1 (`docs/29`). On Linux `interface_number` is enough; on
    /// Windows there is one entry per top-level collection and the usage page
    /// is what tells them apart. If no hint works, it tries each candidate
    /// with the 0x81 read, which is harmless.
    pub fn open_device() -> Res<Keyboard> {
        let api = HidApi::new().map_err(|e| format!("hidapi did not initialize: {e}"))?;
        let mut candidates: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
            .collect();
        if candidates.is_empty() {
            return Err(format!(
                "keyboard {VID:04x}:{PID:04x} not found (plugged in? \
                 permission on /dev/bus/usb?)"
            ));
        }
        // Best hint first: vendor usage page 0xFF01, then interface 1.
        candidates.sort_by_key(|d| (usage_page_hint(d), interface_hint(d)));

        let mut errors = Vec::new();
        for info in candidates {
            let dev = match info.open_device(&api) {
                Ok(d) => d,
                Err(e) => {
                    errors.push(format!("{:?}: {e}", info.path()));
                    continue;
                }
            };
            // On this keyboard the manufacturer string already starts with the
            // product name ("G.SKILL RIPJAWS KM780 RGB ..."), so concatenating
            // the two repeats everything. The product one alone is enough; the
            // manufacturer only comes in if it is missing.
            let name = info
                .product_string()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| info.manufacturer_string().map(str::trim))
                .unwrap_or("KM780 RGB")
                .to_string();
            let kb = Keyboard { dev, description: name.trim().to_string() };
            // Probe with the only harmless read there is.
            match kb.active_mode() {
                Ok(_) => return Ok(kb),
                Err(e) => errors.push(format!("{:?}: {e}", info.path())),
            }
        }
        Err(format!(
            "none of the HID entries accepted Feature report 7:\n{}",
            errors.join("\n")
        ))
    }

    fn build(cmd: u8, mode: u8, b3: u8, b4: u8, b5: u8, payload: Option<&[u8]>) -> Vec<u8> {
        let mut b = vec![0u8; PKT];
        b[0] = REPORT_ID;
        b[1] = cmd;
        b[2] = mode;
        b[3] = b3;
        b[4] = b4;
        b[5] = b5;
        if let Some(p) = payload {
            b[HDR..HDR + p.len()].copy_from_slice(p);
        }
        b
    }

    fn write(&self, packet: &[u8], wait: Duration) -> Res<()> {
        self.dev
            .send_feature_report(packet)
            .map_err(|e| format!("write of command 0x{:02x} failed: {e}", packet[1]))?;
        sleep(wait);
        Ok(())
    }

    /// Sends the read command and collects the answer -- the same
    /// SetFeature/GetFeature pair the official app uses.
    fn read(&self, packet: &[u8], wait: Duration) -> Res<Vec<u8>> {
        self.write(packet, wait)?;
        let mut buf = vec![0u8; PKT];
        buf[0] = REPORT_ID;
        let n = self
            .dev
            .get_feature_report(&mut buf)
            .map_err(|e| format!("read of command 0x{:02x} failed: {e}", packet[1]))?;
        if n < HDR {
            return Err(format!("short response ({n} bytes) to command 0x{:02x}", packet[1]));
        }
        sleep(wait);
        Ok(buf)
    }

    /// Number of the active mode, 1..3.
    ///
    /// Worth re-reading every time: the M1/M2/M3 keys switch the mode **inside
    /// the keyboard**, without telling the host (`docs/25`).
    pub fn active_mode(&self) -> Res<u8> {
        let d = self.read(&Self::build(MODE_GET, 0, 0, 0, 0, None), WAIT_SIMPLE)?;
        Ok(d[2])
    }

    /// Switches the active mode -- the same as pressing M1/M2/M3 on the keyboard.
    ///
    /// `07 01 <mode> 00 ...`, the rest zeroed (`docs/09` §4). It is the only
    /// place in the app where the packet is built from scratch instead of
    /// read-change-write, and that is fine: the command **is** just the mode
    /// number, the following 261 bytes are zero by specification, and there is
    /// no unknown field to preserve.
    pub fn switch_mode(&self, mode: u8) -> Res<()> {
        if !(1..=3).contains(&mode) {
            return Err(format!("mode {mode} outside 1..3"));
        }
        self.write(&Self::build(MODE_SET, mode, 0, 0, 0, None), WAIT_READ)
    }

    /// The 264 bytes of the mode payload.
    pub fn read_mode(&self, mode: u8) -> Res<Vec<u8>> {
        let d = self.read(&Self::build(INFO_GET, mode, 0, 0, 0, None), WAIT_READ)?;
        if d[1] != INFO_GET || d[2] != mode {
            return Err(format!(
                "unexpected response from 0x82: cmd=0x{:02x} mode={}",
                d[1], d[2]
            ));
        }
        Ok(d)
    }

    /// Writes back the bytes read with [`Keyboard::read_mode`], with the status
    /// zeroed the way the official app does.
    ///
    /// Still unused: v1 does not touch brightness or effect. It is here because
    /// it is the pair of `read_mode` and because brightness is the next step
    /// (`docs/29`).
    ///
    /// Always read-change-write: the unknown fields travel intact through the
    /// round-trip, and that is what makes it possible to touch the known
    /// without understanding the rest (`docs/29`, rule 3).
    #[allow(dead_code)]
    pub fn write_mode(&self, mode: u8, d: &[u8]) -> Res<()> {
        if d.len() != PKT {
            return Err(format!("mode payload with {} bytes, expected {PKT}", d.len()));
        }
        let mut b = d.to_vec();
        b[0] = REPORT_ID;
        b[1] = INFO_SET;
        b[2] = mode;
        b[OFF_STATUS] = 0x00;
        self.write(&b, WAIT_READ)
    }

    pub fn read_colors(&self, mode: u8) -> Res<Channels> {
        let mut out: Vec<Vec<u8>> = Vec::with_capacity(3);
        for ch in CHANNELS {
            let d = self.read(&Self::build(LED_GET, mode, ch, 0, 0, None), WAIT_READ)?;
            if d[1] != LED_GET || d[3] != ch {
                return Err(format!(
                    "unexpected response from 0x89: cmd=0x{:02x} channel={}",
                    d[1], d[3]
                ));
            }
            out.push(d[HDR..HDR + crate::slots::NSLOTS].to_vec());
        }
        Ok([out.remove(0), out.remove(0), out.remove(0)])
    }

    /// Three packets, one per channel. There is no commit command: the third
    /// packet closes the update (`docs/09` §6.4). That is why any color change
    /// resends all three -- there is no partial channel write.
    pub fn write_colors(&self, mode: u8, channels: &Channels) -> Res<()> {
        for (ch, data) in CHANNELS.iter().zip(channels.iter()) {
            if data.len() != crate::slots::NSLOTS {
                return Err(format!("channel with {} bytes, expected 144", data.len()));
            }
            self.write(
                &Self::build(LED_SET, mode, *ch, 0, 0, Some(data)),
                WAIT_CHANNEL,
            )?;
        }
        sleep(WAIT_AFTER_COLOR); // the official app's extra wait after the 3rd channel
        Ok(())
    }
}

/// 0 if the entry is the vendor collection 0xFF01, 1 otherwise.
///
/// The libusb backend on Linux does not expose `usage_page` -- `hidapi` itself
/// leaves the method out of the build in that case. There the only hint is the
/// interface, which is enough. On Windows, where there is one entry per
/// top-level collection, the usage page is what tells them apart (`docs/29`).
#[cfg(target_os = "windows")]
fn usage_page_hint(d: &hidapi::DeviceInfo) -> u8 {
    if d.usage_page() == 0xFF01 { 0 } else { 1 }
}

#[cfg(not(target_os = "windows"))]
fn usage_page_hint(_d: &hidapi::DeviceInfo) -> u8 {
    0
}

/// 0 if the entry is interface 1, which is where the command lives (`docs/01`).
fn interface_hint(d: &hidapi::DeviceInfo) -> u8 {
    if d.interface_number() == 1 { 0 } else { 1 }
}
