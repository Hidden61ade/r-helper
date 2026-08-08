> [!WARNING]  
> Note from Fatalution: I returned my Blade 16.    

# R-Helper

A Windows application to control Razer Blade settings w/o Synapse.

<img width="332" height="388" alt="image" src="https://github.com/user-attachments/assets/3a4630d8-d79a-4e6b-b6a6-df4f1f52bdb9" />

## Features

- Performance modes: Battery, Silent, Balanced, Performance, Hyperboost, Custom
- Custom mode: CPU/GPU Low/Medium/High/Boost adjustments with experimental Undervolt option (no idea what it does as it's a preset)
- Fan control: Auto/Manual, with current RPM display
- Keyboard backlight brightness control
- Logo lighting: Static, Breathing, Off (only shown on devices with a lid logo)
- Battery care: Toggle charging threshold (80%)

## Supported Devices

| Model | Model Number Prefix | USB PID |
|-------|--------------------|---------|
| Razer Blade 14” (2022) | RZ09-0427 | 0x028C |
| Razer Blade 15” (2022) | RZ09-0421 | 0x028A |
| Razer Blade 17” (2022) | RZ09-0423 | 0x028B |
| Razer Blade 14” (2023) | RZ09-0482 | 0x029D |
| Razer Blade 15” (2023) | RZ09-0485 | 0x029E |
| Razer Blade 16” (2023) | RZ09-0483 | 0x029F |
| Razer Blade 18” (2023) | RZ09-0484 | 0x02A0 |
| Razer Blade 14” (2024) | RZ09-0508 | 0x02B6 |
| Razer Blade 16” (2024) | RZ09-0510 | 0x02B7 |
| Razer Blade 18” (2024) | RZ09-0509 | 0x02B8 |
| Razer Blade 14” (2025) | RZ09-0530 | 0x02C5 |
| Razer Blade 16” (2025) | RZ09-0528 | 0x02C6 |
| Razer Blade 18” (2025) | RZ09-0529 | 0x02C7 |
| Razer Blade 16” (2026) | RZ09-0581 | 0x02E0 |

If your exact model number is not listed but your laptop shares a USB PID with a
supported model, R-Helper will automatically fall back to that model's profile.

Newer models (2025+) require an initialization command sequence on startup,
which R-Helper sends automatically — Synapse does not need to be installed or
running.


## Installation

1. Download the latest release
2. Run `rhelper.exe`

## Building

```powershell
cargo build --release
```

## Architecture

Core device control via locally vendored `librazer` (derived from razer-ctl)


## License

MIT. Includes MIT-licensed portions derived from razer-ctl (see NOTICE and THIRD_PARTY_LICENSES.md).

## Support

If you really want to express gratitude: [PayPal Donation](https://www.paypal.com/paypalme/fatalutionDE)
