//! Diagnostic tool: detects the laptop, runs the descriptor's init sequence,
//! and reads back basic state. Useful when adding support for a new model.
//!
//! Run with: cargo run -p librazer --example detect

use librazer::device::Device;
use librazer::{command, types::FanZone};

fn main() -> anyhow::Result<()> {
    let (pids, model) = Device::enumerate()?;
    println!("BIOS model number: {}", model);
    println!("Razer USB PIDs:    {:04x?}", pids);

    let device = Device::detect()?;
    println!("Matched device:    {} (PID 0x{:04x})", device.info().name, device.info().pid);
    println!("Features:          {:?}", device.info().features);

    let (perf_mode, fan_mode) = command::get_perf_mode(&device)?;
    println!("Performance mode:  {:?} (fan {:?})", perf_mode, fan_mode);

    match command::get_fan_actual_rpm(&device, FanZone::Zone1) {
        Ok(rpm) => println!("Fan 1 actual RPM:  {}", rpm),
        Err(e) => println!("Fan 1 actual RPM:  read failed: {}", e),
    }

    match command::get_battery_care(&device) {
        Ok(bho) => println!("Battery care:      {:?}", bho),
        Err(e) => println!("Battery care:      read failed: {}", e),
    }

    match command::get_keyboard_brightness(&device) {
        Ok(b) => println!("Kbd brightness:    {}", b),
        Err(e) => println!("Kbd brightness:    read failed: {}", e),
    }

    Ok(())
}
