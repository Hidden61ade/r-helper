//! Diagnostic tool: captures what the M1-M5 (and other special) keys emit.
//!
//! Listens simultaneously on:
//!   1. all Razer HID interfaces (raw input reports, hex-dumped)
//!   2. system-level virtual keys (F13-F24, volume/media keys)
//!
//! Usage:
//!   cargo run -p librazer --example keycap [seconds]        # capture (default 30s)
//!   cargo run -p librazer --example keycap -- driver        # set device driver mode (0x03)
//!   cargo run -p librazer --example keycap -- normal        # set device normal mode (0x00)

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vk: i32) -> i16;
}

const RAZER_VID: u16 = 0x1532;

fn vk_name(vk: i32) -> String {
    match vk {
        0x7C..=0x87 => format!("F{}", vk - 0x7C + 13),
        0xAD => "VOLUME_MUTE".into(),
        0xAE => "VOLUME_DOWN".into(),
        0xAF => "VOLUME_UP".into(),
        0xB0 => "MEDIA_NEXT".into(),
        0xB1 => "MEDIA_PREV".into(),
        0xB2 => "MEDIA_STOP".into(),
        0xB3 => "MEDIA_PLAY_PAUSE".into(),
        0xB4 => "LAUNCH_MAIL".into(),
        0xB5 => "LAUNCH_MEDIA_SELECT".into(),
        0xB6 => "LAUNCH_APP1".into(),
        0xB7 => "LAUNCH_APP2".into(),
        _ => format!("VK_{:02X}", vk),
    }
}

fn main() -> anyhow::Result<()> {
    let arg = std::env::args().nth(1);

    // Mode switch sub-commands
    if let Some(mode) = arg.as_deref().and_then(|a| match a {
        "driver" => Some(0x03u8),
        "normal" => Some(0x00u8),
        _ => None,
    }) {
        let device = librazer::device::Device::detect()?;
        librazer::command::send_command(&device, 0x0004, &[mode, 0])?;
        println!("Device mode set to 0x{:02x}", mode);
        return Ok(());
    }

    let seconds: u64 = arg.and_then(|a| a.parse().ok()).unwrap_or(30);

    // Show the current device mode (same command as "lights always on")
    match librazer::device::Device::detect() {
        Ok(device) => match librazer::command::send_command(&device, 0x0084, &[0, 0]) {
            Ok(response) => println!(
                "Current device mode: 0x{:02x} (0x00 = normal, 0x03 = driver)",
                response.get_args()[0]
            ),
            Err(e) => println!("Could not read device mode: {}", e),
        },
        Err(e) => println!("Razer device not detected: {}", e),
    }

    let api = hidapi::HidApi::new()?;
    let stop = Arc::new(AtomicBool::new(false));
    let mut threads = Vec::new();

    println!("\nRazer HID interfaces:");
    for info in api.device_list().filter(|i| i.vendor_id() == RAZER_VID) {
        let label = format!(
            "if{} usage_page=0x{:04x} usage=0x{:04x}",
            info.interface_number(),
            info.usage_page(),
            info.usage()
        );
        match api.open_path(info.path()) {
            Ok(dev) => {
                println!("  [open] {}", label);
                let stop = stop.clone();
                threads.push(std::thread::spawn(move || {
                    let mut buf = [0u8; 64];
                    while !stop.load(Ordering::Relaxed) {
                        match dev.read_timeout(&mut buf, 200) {
                            Ok(n) if n > 0 => {
                                let hex: Vec<String> =
                                    buf[..n].iter().map(|b| format!("{:02x}", b)).collect();
                                println!("HID {} -> {}", label, hex.join(" "));
                                let _ = std::io::stdout().flush();
                            }
                            _ => {}
                        }
                    }
                }));
            }
            Err(_) => println!("  [skip] {} (no read access)", label),
        }
    }

    println!("\nCapturing for {} seconds - press M1-M5 (and other special keys) now...", seconds);

    // Poll system-level virtual keys for transitions
    #[cfg(target_os = "windows")]
    {
        let interesting: Vec<i32> = (0x7C..=0x87).chain(0xAD..=0xB7).collect();
        let mut last_state = vec![false; interesting.len()];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        while std::time::Instant::now() < deadline {
            for (idx, &vk) in interesting.iter().enumerate() {
                let down = unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000 != 0;
                if down && !last_state[idx] {
                    println!("KEY {} pressed", vk_name(vk));
                    let _ = std::io::stdout().flush();
                }
                last_state[idx] = down;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
    #[cfg(not(target_os = "windows"))]
    std::thread::sleep(std::time::Duration::from_secs(seconds));

    stop.store(true, Ordering::Relaxed);
    for t in threads {
        let _ = t.join();
    }
    println!("Capture finished.");
    Ok(())
}
