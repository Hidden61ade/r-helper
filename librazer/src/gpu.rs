//! System-level GPU rendering preference (NVIDIA Optimus) control via the
//! NVIDIA driver settings API (DRS), plus detection of which GPU currently
//! drives the primary display.
//!
//! Note: this is not part of the Razer USB HID protocol. True display MUX
//! switching (NVIDIA "Advanced Optimus" display modes) has no public API and
//! is only available through the NVIDIA Control Panel. What we can control
//! programmatically is the global "Preferred graphics processor" setting,
//! which decides where applications render by default.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPreference {
    /// Render on the integrated GPU by default (power saving)
    Integrated,
    /// Let the driver decide per application (NVIDIA default)
    Auto,
    /// Render on the discrete NVIDIA GPU by default (performance)
    Dedicated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOwner {
    /// The primary display is driven by the integrated GPU (hybrid mode)
    Integrated,
    /// The primary display is driven by the NVIDIA GPU (dGPU-only mode)
    Nvidia,
    Unknown,
}

#[cfg(target_os = "windows")]
pub use imp::{get_display_owner, get_gpu_preference, set_gpu_preference};

#[cfg(not(target_os = "windows"))]
mod stub {
    use super::*;
    use anyhow::Result;
    pub fn get_display_owner() -> DisplayOwner {
        DisplayOwner::Unknown
    }
    pub fn get_gpu_preference() -> Result<GpuPreference> {
        anyhow::bail!("GPU preference control is only implemented for Windows")
    }
    pub fn set_gpu_preference(_pref: GpuPreference) -> Result<()> {
        anyhow::bail!("GPU preference control is only implemented for Windows")
    }
}
#[cfg(not(target_os = "windows"))]
pub use stub::{get_display_owner, get_gpu_preference, set_gpu_preference};

#[cfg(target_os = "windows")]
mod imp {
    use super::{DisplayOwner, GpuPreference};
    use anyhow::{bail, Result};
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(lplibfilename: *const u16) -> *mut c_void;
        fn GetProcAddress(hmodule: *mut c_void, lpprocname: *const u8) -> *mut c_void;
    }

    // ---------------------------------------------------------------------
    // Primary display owner detection (EnumDisplayDevicesW)
    // ---------------------------------------------------------------------

    #[repr(C)]
    struct DisplayDeviceW {
        cb: u32,
        device_name: [u16; 32],
        device_string: [u16; 128],
        state_flags: u32,
        device_id: [u16; 128],
        device_key: [u16; 128],
    }

    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplayDevicesW(
            lpdevice: *const u16,
            idevnum: u32,
            lpdisplaydevice: *mut DisplayDeviceW,
            dwflags: u32,
        ) -> i32;
    }

    const DISPLAY_DEVICE_ATTACHED_TO_DESKTOP: u32 = 0x1;
    const DISPLAY_DEVICE_PRIMARY_DEVICE: u32 = 0x4;
    const DISPLAY_DEVICE_MIRRORING_DRIVER: u32 = 0x8;

    fn adapter_to_owner(device_string: &[u16]) -> DisplayOwner {
        let name = String::from_utf16_lossy(device_string).to_ascii_uppercase();
        if name.contains("NVIDIA") {
            DisplayOwner::Nvidia
        } else if name.contains("AMD")
            || name.contains("RADEON")
            || name.contains("INTEL")
            || name.contains("IRIS")
        {
            DisplayOwner::Integrated
        } else {
            DisplayOwner::Unknown
        }
    }

    pub fn get_display_owner() -> DisplayOwner {
        let mut fallback = DisplayOwner::Unknown;
        for i in 0u32.. {
            let mut dd: DisplayDeviceW = unsafe { std::mem::zeroed() };
            dd.cb = std::mem::size_of::<DisplayDeviceW>() as u32;
            if unsafe { EnumDisplayDevicesW(std::ptr::null(), i, &mut dd, 0) } == 0 {
                break;
            }
            if dd.state_flags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0 {
                continue;
            }
            if dd.state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
                continue;
            }
            let owner = adapter_to_owner(&dd.device_string);
            if dd.state_flags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0 {
                return owner;
            }
            if fallback == DisplayOwner::Unknown {
                fallback = owner;
            }
        }
        fallback
    }

    // ---------------------------------------------------------------------
    // NVIDIA DRS (driver settings) via nvapi64.dll
    // ---------------------------------------------------------------------

    const NVAPI_OK: i32 = 0;

    // Function IDs from the public nvapi interface table
    const ID_INITIALIZE: u32 = 0x0150E828;
    const ID_DRS_CREATE_SESSION: u32 = 0x0694D52E;
    const ID_DRS_DESTROY_SESSION: u32 = 0xDAD9CFF8;
    const ID_DRS_LOAD_SETTINGS: u32 = 0x375DBD6B;
    const ID_DRS_SAVE_SETTINGS: u32 = 0xFCBC7E14;
    const ID_DRS_GET_BASE_PROFILE: u32 = 0xDA8466A0;
    const ID_DRS_GET_SETTING: u32 = 0x73BF8338;
    const ID_DRS_SET_SETTING: u32 = 0x577DD202;

    // Setting IDs and values from NvApiDriverSettings.h ("Preferred graphics
    // processor" is the combination of SHIM_MCCOMPAT and SHIM_RENDERING_MODE)
    const SHIM_MCCOMPAT_ID: u32 = 0x10F9DC80;
    const SHIM_RENDERING_MODE_ID: u32 = 0x10F9DC81;
    const SHIM_VALUE_INTEGRATED: u32 = 0x0000_0000;
    const SHIM_VALUE_ENABLE: u32 = 0x0000_0001;
    const SHIM_VALUE_AUTO_SELECT: u32 = 0x0000_0010;
    const NVDRS_DWORD_TYPE: u32 = 0;

    #[repr(C)]
    struct NvdrsSetting {
        version: u32,
        setting_name: [u16; 2048],
        setting_id: u32,
        setting_type: u32,
        setting_location: u32,
        is_current_predefined: u32,
        is_predefined_valid: u32,
        // Unions of (u32 | NvAPI_UnicodeString | NVDRS_BINARY_SETTING); for
        // DWORD settings only the first 4 bytes are used.
        predefined_value: [u8; 4100],
        current_value: [u8; 4100],
    }

    const NVDRS_SETTING_VER: u32 = (std::mem::size_of::<NvdrsSetting>() as u32) | (1 << 16);

    type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *mut c_void;
    type InitializeFn = unsafe extern "C" fn() -> i32;
    type CreateSessionFn = unsafe extern "C" fn(*mut *mut c_void) -> i32;
    type SessionOpFn = unsafe extern "C" fn(*mut c_void) -> i32;
    type GetBaseProfileFn = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
    type GetSettingFn = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut NvdrsSetting) -> i32;
    type SetSettingFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut NvdrsSetting) -> i32;

    fn nv_check(status: i32, what: &str) -> Result<()> {
        if status == NVAPI_OK {
            Ok(())
        } else {
            bail!("{} failed with NVAPI status {}", what, status)
        }
    }

    struct Drs {
        query: QueryInterfaceFn,
        session: *mut c_void,
        profile: *mut c_void,
    }

    impl Drs {
        fn interface(&self, id: u32, name: &str) -> Result<*mut c_void> {
            let ptr = unsafe { (self.query)(id) };
            if ptr.is_null() {
                bail!("NVAPI interface {} (0x{:08x}) is unavailable", name, id);
            }
            Ok(ptr)
        }

        fn open() -> Result<Drs> {
            unsafe {
                let name: Vec<u16> = "nvapi64.dll".encode_utf16().chain([0]).collect();
                let lib = LoadLibraryW(name.as_ptr());
                if lib.is_null() {
                    bail!("nvapi64.dll not found (NVIDIA driver not installed?)");
                }
                let qi = GetProcAddress(lib, b"nvapi_QueryInterface\0".as_ptr());
                if qi.is_null() {
                    bail!("nvapi_QueryInterface export not found in nvapi64.dll");
                }
                let query: QueryInterfaceFn = std::mem::transmute(qi);

                let mut drs = Drs { query, session: std::ptr::null_mut(), profile: std::ptr::null_mut() };

                let init: InitializeFn =
                    std::mem::transmute(drs.interface(ID_INITIALIZE, "NvAPI_Initialize")?);
                nv_check(init(), "NvAPI_Initialize")?;

                let create: CreateSessionFn = std::mem::transmute(
                    drs.interface(ID_DRS_CREATE_SESSION, "NvAPI_DRS_CreateSession")?,
                );
                nv_check(create(&mut drs.session), "NvAPI_DRS_CreateSession")?;

                let load: SessionOpFn = std::mem::transmute(
                    drs.interface(ID_DRS_LOAD_SETTINGS, "NvAPI_DRS_LoadSettings")?,
                );
                nv_check(load(drs.session), "NvAPI_DRS_LoadSettings")?;

                let base: GetBaseProfileFn = std::mem::transmute(
                    drs.interface(ID_DRS_GET_BASE_PROFILE, "NvAPI_DRS_GetBaseProfile")?,
                );
                nv_check(base(drs.session, &mut drs.profile), "NvAPI_DRS_GetBaseProfile")?;

                Ok(drs)
            }
        }

        /// Returns the current DWORD value of a setting, or None if the
        /// setting is not present in the base profile (driver default).
        fn get(&self, setting_id: u32) -> Result<Option<u32>> {
            unsafe {
                let getter: GetSettingFn = std::mem::transmute(
                    self.interface(ID_DRS_GET_SETTING, "NvAPI_DRS_GetSetting")?,
                );
                let mut setting: Box<NvdrsSetting> = Box::new(std::mem::zeroed());
                setting.version = NVDRS_SETTING_VER;
                let status = getter(self.session, self.profile, setting_id, &mut *setting);
                if status != NVAPI_OK {
                    // Typically "setting not found": the base profile carries
                    // no explicit value, meaning the driver default applies.
                    return Ok(None);
                }
                let value = u32::from_ne_bytes(setting.current_value[..4].try_into().unwrap());
                Ok(Some(value))
            }
        }

        fn set(&self, setting_id: u32, value: u32) -> Result<()> {
            unsafe {
                let setter: SetSettingFn = std::mem::transmute(
                    self.interface(ID_DRS_SET_SETTING, "NvAPI_DRS_SetSetting")?,
                );
                let mut setting: Box<NvdrsSetting> = Box::new(std::mem::zeroed());
                setting.version = NVDRS_SETTING_VER;
                setting.setting_id = setting_id;
                setting.setting_type = NVDRS_DWORD_TYPE;
                setting.current_value[..4].copy_from_slice(&value.to_ne_bytes());
                nv_check(
                    setter(self.session, self.profile, &mut *setting),
                    "NvAPI_DRS_SetSetting",
                )
            }
        }

        fn save(&self) -> Result<()> {
            unsafe {
                let save: SessionOpFn = std::mem::transmute(
                    self.interface(ID_DRS_SAVE_SETTINGS, "NvAPI_DRS_SaveSettings")?,
                );
                nv_check(save(self.session), "NvAPI_DRS_SaveSettings")
            }
        }
    }

    impl Drop for Drs {
        fn drop(&mut self) {
            if !self.session.is_null() {
                unsafe {
                    if let Ok(ptr) =
                        self.interface(ID_DRS_DESTROY_SESSION, "NvAPI_DRS_DestroySession")
                    {
                        let destroy: SessionOpFn = std::mem::transmute(ptr);
                        destroy(self.session);
                    }
                }
            }
        }
    }

    fn value_to_preference(value: u32) -> GpuPreference {
        if value & SHIM_VALUE_AUTO_SELECT != 0 {
            GpuPreference::Auto
        } else if value & SHIM_VALUE_ENABLE != 0 {
            GpuPreference::Dedicated
        } else {
            GpuPreference::Integrated
        }
    }

    pub fn get_gpu_preference() -> Result<GpuPreference> {
        let drs = Drs::open()?;
        match drs.get(SHIM_RENDERING_MODE_ID)? {
            // Driver default is auto-select
            None => Ok(GpuPreference::Auto),
            Some(value) => Ok(value_to_preference(value)),
        }
    }

    pub fn set_gpu_preference(pref: GpuPreference) -> Result<()> {
        let value = match pref {
            GpuPreference::Integrated => SHIM_VALUE_INTEGRATED,
            GpuPreference::Auto => SHIM_VALUE_AUTO_SELECT,
            GpuPreference::Dedicated => SHIM_VALUE_ENABLE,
        };
        let drs = Drs::open()?;
        drs.set(SHIM_MCCOMPAT_ID, value)?;
        drs.set(SHIM_RENDERING_MODE_ID, value)?;
        drs.save()
    }
}
