use eframe::egui;
use librazer::gpu::{DisplayOwner, GpuPreference};

/// Actions that can be triggered from the GPU section UI
#[derive(Debug, Clone, PartialEq)]
pub enum GpuAction {
    None,
    /// Set the global rendering preference (NVIDIA "Preferred graphics processor")
    SetPreference(GpuPreference),
    /// Open the Windows per-app graphics preference settings page
    OpenWindowsGraphics,
    /// Open the NVIDIA Control Panel (for Advanced Optimus display mode)
    OpenNvidiaPanel,
}

/// Renders the GPU section: current display owner (MUX state), the global
/// rendering preference selector, and shortcuts to the system tools that
/// control what this app cannot (per-app overrides, display MUX mode).
pub fn render_gpu_section(
    ui: &mut egui::Ui,
    preference: Option<GpuPreference>,
    display_owner: DisplayOwner,
) -> GpuAction {
    let mut action = GpuAction::None;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("🖳 GPU").selectable(false));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (text, color) = match display_owner {
                    DisplayOwner::Nvidia => {
                        ("Display: NVIDIA dGPU", egui::Color32::from_rgb(118, 185, 0))
                    }
                    DisplayOwner::Integrated => ("Display: iGPU", egui::Color32::LIGHT_BLUE),
                    DisplayOwner::Unknown => ("Display: Unknown", egui::Color32::GRAY),
                };
                ui.add(egui::Label::new(egui::RichText::new(text).color(color)).selectable(false))
                    .on_hover_text(
                        "Which GPU currently drives the primary display (MUX state).\n\
                         Switching the display MUX itself is only possible in\n\
                         NVIDIA Control Panel → Manage Display Mode.",
                    );
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Render Preference:").selectable(false));

            match preference {
                Some(current) => {
                    let entries = [
                        (
                            GpuPreference::Integrated,
                            "Integrated",
                            "Applications render on the integrated GPU by default (best battery life)",
                        ),
                        (
                            GpuPreference::Auto,
                            "Auto",
                            "The NVIDIA driver picks the GPU per application (driver default)",
                        ),
                        (
                            GpuPreference::Dedicated,
                            "NVIDIA",
                            "Applications render on the NVIDIA GPU by default (best performance)",
                        ),
                    ];
                    for (pref, label, tip) in entries {
                        let selected = current == pref;
                        let response = ui.selectable_label(selected, label).on_hover_text(tip);
                        if response.clicked() && !selected {
                            action = GpuAction::SetPreference(pref);
                        }
                    }
                }
                None => {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Unavailable (NVIDIA driver not detected)")
                                .italics()
                                .color(egui::Color32::GRAY),
                        )
                        .selectable(false),
                    );
                }
            }
        });

        ui.horizontal(|ui| {
            if ui
                .small_button("Windows Graphics Settings")
                .on_hover_text("Per-app GPU preferences set here override the global preference")
                .clicked()
            {
                action = GpuAction::OpenWindowsGraphics;
            }
            if ui
                .small_button("NVIDIA Control Panel")
                .on_hover_text(
                    "Manage Display Mode there to switch the display MUX\n\
                     (Optimus / NVIDIA GPU only / Automatic)",
                )
                .clicked()
            {
                action = GpuAction::OpenNvidiaPanel;
            }
        });
    });

    action
}
