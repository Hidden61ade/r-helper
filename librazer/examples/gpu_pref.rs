//! Diagnostic tool for the GPU rendering preference control.
//!
//! Usage:
//!   cargo run -p librazer --example gpu_pref            # show current state
//!   cargo run -p librazer --example gpu_pref integrated # set iGPU preference
//!   cargo run -p librazer --example gpu_pref auto       # set auto-select
//!   cargo run -p librazer --example gpu_pref dedicated  # set NVIDIA dGPU

use librazer::gpu::{self, GpuPreference};

fn main() -> anyhow::Result<()> {
    if let Some(arg) = std::env::args().nth(1) {
        let pref = match arg.as_str() {
            "integrated" => GpuPreference::Integrated,
            "auto" => GpuPreference::Auto,
            "dedicated" => GpuPreference::Dedicated,
            other => anyhow::bail!("unknown preference '{}'", other),
        };
        gpu::set_gpu_preference(pref)?;
        println!("Preference set to {:?}", pref);
    }

    println!("Primary display owner: {:?}", gpu::get_display_owner());
    println!("Rendering preference:  {:?}", gpu::get_gpu_preference()?);
    Ok(())
}
