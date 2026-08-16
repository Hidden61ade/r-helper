//! Background listener for the dedicated M1-M5 macro keys on Razer Blade
//! laptops.
//!
//! The macro keys have no firmware-level function: they only emit events when
//! the device is in "driver mode" (device mode 0x03, the same flag exposed in
//! this crate as [`crate::types::LightsAlwaysOn`]). In that mode the EC sends
//! a HID input report on a vendor collection of USB interface 1:
//!
//! ```text
//! report id 0x04, key code, 0, 0, ...   (key press)
//! report id 0x04, 0x00,     0, 0, ...   (key release)
//! ```
//!
//! Key codes captured on a Razer Blade 16 (2025): M3=0x03, M4=0xd3, M5=0xd4.
//! M1 and M2 never emit vendor events: their firmware sends regular
//! Page Up / Page Down keystrokes, so they work without any software.
//! Razer Synapse interprets the vendor events in software; this module lets
//! the application do the same.

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

const RAZER_VID: u16 = 0x1532;
const MACRO_REPORT_ID: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroKey {
    M3 = 0,
    M4 = 1,
    M5 = 2,
}

impl MacroKey {
    pub fn from_code(code: u8) -> Option<MacroKey> {
        match code {
            0x03 => Some(MacroKey::M3),
            0xd3 => Some(MacroKey::M4),
            0xd4 => Some(MacroKey::M5),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MacroKey::M3 => "M3",
            MacroKey::M4 => "M4",
            MacroKey::M5 => "M5",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroKeyEvent {
    /// A recognized macro key was pressed.
    Pressed(MacroKey),
    /// An unrecognized key code arrived on the macro report. Surfacing it
    /// helps map keys on models with different codes.
    Unknown(u8),
}

/// Reads macro key input reports on background threads and forwards events
/// through an mpsc channel. Dropping the listener stops the threads.
pub struct MacroKeyListener {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl MacroKeyListener {
    pub fn start() -> Result<(MacroKeyListener, mpsc::Receiver<MacroKeyEvent>)> {
        let api = hidapi::HidApi::new().context("Failed to create hid api")?;
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();

        // The macro key events arrive on a vendor collection of interface 1
        // (usage_page 0x0001, usage 0x0000). There can be more than one such
        // collection; listen on all of them.
        for info in api.device_list().filter(|i| {
            i.vendor_id() == RAZER_VID
                && i.interface_number() == 1
                && i.usage_page() == 0x0001
                && i.usage() == 0x0000
        }) {
            let Ok(device) = api.open_path(info.path()) else {
                continue;
            };
            let tx = tx.clone();
            let stop = stop.clone();
            threads.push(std::thread::spawn(move || {
                let mut buf = [0u8; 32];
                while !stop.load(Ordering::Relaxed) {
                    match device.read_timeout(&mut buf, 200) {
                        Ok(n) if n >= 2 && buf[0] == MACRO_REPORT_ID && buf[1] != 0 => {
                            let event = match MacroKey::from_code(buf[1]) {
                                Some(key) => MacroKeyEvent::Pressed(key),
                                None => MacroKeyEvent::Unknown(buf[1]),
                            };
                            if tx.send(event).is_err() {
                                return;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => return,
                    }
                }
            }));
        }

        anyhow::ensure!(!threads.is_empty(), "No macro key HID interface found");
        Ok((MacroKeyListener { stop, threads }, rx))
    }
}

impl Drop for MacroKeyListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
