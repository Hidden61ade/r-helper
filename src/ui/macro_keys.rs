use eframe::egui;

/// What a macro key does when pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroAction {
    Disabled,
    PageUp,
    PageDown,
    CycleRefreshRate,
    CyclePerfMode,
    ToggleMicMute,
}

impl MacroAction {
    pub const ALL: [MacroAction; 6] = [
        MacroAction::Disabled,
        MacroAction::PageUp,
        MacroAction::PageDown,
        MacroAction::CycleRefreshRate,
        MacroAction::CyclePerfMode,
        MacroAction::ToggleMicMute,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MacroAction::Disabled => "Disabled",
            MacroAction::PageUp => "Page Up",
            MacroAction::PageDown => "Page Down",
            MacroAction::CycleRefreshRate => "Cycle Refresh Rate",
            MacroAction::CyclePerfMode => "Cycle Performance Mode",
            MacroAction::ToggleMicMute => "Toggle Microphone Mute",
        }
    }
}

/// Actions that can be triggered from the macro keys section UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroKeysUiAction {
    None,
    SetEnabled(bool),
    /// (key index 0..=4, new assignment)
    Assign(usize, MacroAction),
    /// Turn on driver mode (= "Backlight Always On"), required for key events
    EnableDriverMode,
}

const KEY_LABELS: [&str; 3] = ["M3", "M4", "M5"];

pub fn render_macro_keys_section(
    ui: &mut egui::Ui,
    enabled: bool,
    assignments: &[MacroAction; 3],
    driver_mode_active: bool,
    device_present: bool,
) -> MacroKeysUiAction {
    let mut action = MacroKeysUiAction::None;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("⌨ Macro Keys").selectable(false));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut enabled_mut = enabled;
                if ui
                    .checkbox(&mut enabled_mut, "Enabled")
                    .on_hover_text(
                        "Handle the M1-M5 keys in R-Helper (replaces Razer Synapse).\n\
                         Requires driver mode, which is the same device flag as\n\
                         'Keyboard Backlight Always On'.",
                    )
                    .changed()
                {
                    action = MacroKeysUiAction::SetEnabled(enabled_mut);
                }
            });
        });
        ui.separator();

        if !device_present {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("No device connected")
                        .italics()
                        .color(egui::Color32::GRAY),
                )
                .selectable(false),
            );
            return;
        }

        if enabled && !driver_mode_active {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new("⚠ Keys are silent in normal mode")
                            .color(egui::Color32::from_rgb(230, 180, 60)),
                    )
                    .selectable(false),
                )
                .on_hover_text(
                    "The M1-M5 keys only emit events in driver mode.\n\
                     Enabling it also turns on 'Keyboard Backlight Always On'.",
                );
                if ui.small_button("Enable driver mode").clicked() {
                    action = MacroKeysUiAction::EnableDriverMode;
                }
            });
            ui.separator();
        }

        ui.add(
            egui::Label::new(
                egui::RichText::new("M1 / M2 are handled by the keyboard firmware (Page Up / Down)")
                    .small()
                    .color(egui::Color32::GRAY),
            )
            .selectable(false),
        );

        for (index, key_label) in KEY_LABELS.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [28.0_f32, ui.available_height()],
                        egui::Label::new(*key_label).selectable(false),
                    );
                    let mut current = assignments[index];
                    egui::ComboBox::from_id_salt(("macro_key_assign", index))
                        .width(220.0_f32)
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            for option in MacroAction::ALL {
                                if ui
                                    .selectable_value(&mut current, option, option.label())
                                    .clicked()
                                    && current != assignments[index]
                                {
                                    action = MacroKeysUiAction::Assign(index, current);
                                }
                            }
                        });
                });
            });
        }
    });

    action
}
