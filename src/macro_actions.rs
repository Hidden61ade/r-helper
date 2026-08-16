//! System-level actions that macro keys can trigger (Windows only).

#[cfg(windows)]
pub use imp::{cycle_refresh_rate, send_page_key, toggle_mic_mute};

#[cfg(not(windows))]
pub use stub::{cycle_refresh_rate, send_page_key, toggle_mic_mute};

#[cfg(not(windows))]
mod stub {
    use anyhow::Result;

    pub fn send_page_key(_up: bool) -> Result<()> {
        anyhow::bail!("Only implemented for Windows")
    }
    pub fn cycle_refresh_rate() -> Result<u32> {
        anyhow::bail!("Only implemented for Windows")
    }
    pub fn toggle_mic_mute() -> Result<bool> {
        anyhow::bail!("Only implemented for Windows")
    }
}

#[cfg(windows)]
mod imp {
    use anyhow::{bail, Result};

    /// Injects a Page Up (`up == true`) or Page Down key press into the
    /// currently focused application.
    pub fn send_page_key(up: bool) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_NEXT, VK_PRIOR,
        };

        let vk = if up { VK_PRIOR } else { VK_NEXT };
        let key_input = |flags: KEYBD_EVENT_FLAGS| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
            },
        };
        let inputs = [key_input(KEYBD_EVENT_FLAGS(0)), key_input(KEYEVENTF_KEYUP)];
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            bail!("SendInput failed");
        }
        Ok(())
    }

    /// Switches the primary display to the next available refresh rate at the
    /// current resolution (wrapping around), returns the new rate in Hz.
    pub fn cycle_refresh_rate() -> Result<u32> {
        use windows::Win32::Graphics::Gdi::{
            ChangeDisplaySettingsW, EnumDisplaySettingsW, CDS_UPDATEREGISTRY, DEVMODEW,
            DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT,
            DM_PELSWIDTH, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE,
        };

        unsafe {
            let mut current: DEVMODEW = std::mem::zeroed();
            current.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if !EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut current).as_bool() {
                bail!("Failed to read the current display mode");
            }

            let mut rates: Vec<u32> = Vec::new();
            let mut index = 0u32;
            loop {
                let mut mode: DEVMODEW = std::mem::zeroed();
                mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                if !EnumDisplaySettingsW(None, ENUM_DISPLAY_SETTINGS_MODE(index), &mut mode)
                    .as_bool()
                {
                    break;
                }
                index += 1;
                if mode.dmPelsWidth == current.dmPelsWidth
                    && mode.dmPelsHeight == current.dmPelsHeight
                    && mode.dmBitsPerPel == current.dmBitsPerPel
                    && mode.dmDisplayFrequency > 1
                    && !rates.contains(&mode.dmDisplayFrequency)
                {
                    rates.push(mode.dmDisplayFrequency);
                }
            }
            rates.sort_unstable();
            if rates.len() < 2 {
                bail!("Only one refresh rate is available at the current resolution");
            }

            let next = rates
                .iter()
                .copied()
                .find(|&rate| rate > current.dmDisplayFrequency)
                .unwrap_or(rates[0]);

            let mut target = current;
            target.dmDisplayFrequency = next;
            target.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;
            let status = ChangeDisplaySettingsW(Some(&target), CDS_UPDATEREGISTRY);
            if status != DISP_CHANGE_SUCCESSFUL {
                bail!("ChangeDisplaySettings failed with status {:?}", status);
            }
            Ok(next)
        }
    }

    /// Toggles mute on the default capture device (the built-in microphone
    /// unless another input is set as default). Returns the new muted state.
    pub fn toggle_mic_mute() -> Result<bool> {
        use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
        use windows::Win32::Media::Audio::{eCapture, eConsole, IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        };

        unsafe {
            // May already be initialized by the GUI framework; ignore the result.
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            let muted = volume.GetMute()?.as_bool();
            volume.SetMute(!muted, std::ptr::null())?;
            Ok(!muted)
        }
    }
}
