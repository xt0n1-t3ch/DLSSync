//! Device-class taxonomy. Maps the raw class token reported by WMI
//! (`Win32_PnPSignedDriver.DeviceClass`) or WUA (`IWindowsDriverUpdate.DriverClass`)
//! onto a small, UI-friendly set. Matching is case-insensitive and
//! substring-based so vendor variants ("AudioEndpoint", "MEDIA") collapse.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Audio,
    Display,
    Monitor,
    Network,
    Bluetooth,
    Input,
    Storage,
    Printer,
    Camera,
    Sensor,
    Battery,
    SmartCard,
    Firmware,
    Chipset,
    System,
    Usb,
    Other,
}

impl DeviceClass {
    /// Human label for the UI section header.
    pub fn label(self) -> &'static str {
        match self {
            DeviceClass::Audio => "Audio",
            DeviceClass::Display => "Display",
            DeviceClass::Monitor => "Monitors",
            DeviceClass::Network => "Network",
            DeviceClass::Bluetooth => "Bluetooth",
            DeviceClass::Input => "Keyboard & Mouse",
            DeviceClass::Storage => "Storage",
            DeviceClass::Printer => "Printers",
            DeviceClass::Camera => "Cameras & Imaging",
            DeviceClass::Sensor => "Sensors",
            DeviceClass::Battery => "Battery & Power",
            DeviceClass::SmartCard => "Smart Card Readers",
            DeviceClass::Firmware => "Firmware",
            DeviceClass::Chipset => "Chipset",
            DeviceClass::System => "System",
            DeviceClass::Usb => "USB Controllers",
            DeviceClass::Other => "Other Components",
        }
    }
}

/// Best-effort classification from a WUA driver's class token *and* its title.
/// WUA's `DriverClass` is frequently empty or unhelpful for software-component
/// / extension drivers (AudioProcessingObject, Nahimic, generic Monitor INF),
/// so when the class token yields `Other` we re-classify from the human title
/// ("A-Volute AudioProcessingObject…" → Audio, "Dell — Monitor — …" → Display).
pub fn classify_best(class_token: &str, title: &str) -> DeviceClass {
    let by_class = classify(class_token);
    if by_class == DeviceClass::Audio && is_capture_device(title) {
        return DeviceClass::Camera;
    }
    if by_class != DeviceClass::Other {
        return by_class;
    }
    classify(title)
}

/// True when free text names a TV tuner / capture card / camera — used to keep
/// `MEDIA`-class capture hardware out of the Audio bucket.
fn is_capture_device(text: &str) -> bool {
    let s = text.to_ascii_uppercase();
    ["TUNER", "CAPTURE", "WEBCAM", "CAMERA"]
        .iter()
        .any(|t| s.contains(t))
}

/// Classify a raw class token. Order matters: more specific tokens are tested
/// before broad ones (e.g. an HID keyboard is `Input`, not `System`).
pub fn classify(raw: &str) -> DeviceClass {
    let s = raw.trim().to_ascii_uppercase();
    let has = |needle: &str| s.contains(needle);

    let is_capture =
        has("TUNER") || has("CAPTURE") || has("CAMERA") || has("WEBCAM") || has("IMAGE");

    if has("MONITOR") {
        DeviceClass::Monitor
    } else if (has("AUDIO") || s == "SOUND") || (has("MEDIA") && !is_capture) {
        DeviceClass::Audio
    } else if has("BLUETOOTH") {
        DeviceClass::Bluetooth
    } else if has("DISPLAY") {
        DeviceClass::Display
    } else if has("HID") || has("KEYBOARD") || has("MOUSE") {
        DeviceClass::Input
    } else if has("PRINT") {
        DeviceClass::Printer
    } else if is_capture {
        DeviceClass::Camera
    } else if has("SMARTCARD") || has("SMART CARD") {
        DeviceClass::SmartCard
    } else if has("BIOMETRIC") || has("SENSOR") {
        DeviceClass::Sensor
    } else if has("BATTERY") {
        DeviceClass::Battery
    } else if has("FIRMWARE") {
        DeviceClass::Firmware
    } else if has("NET") {
        DeviceClass::Network
    } else if has("USB") {
        DeviceClass::Usb
    } else if has("DISK")
        || has("SCSI")
        || has("STORAGE")
        || has("HDC")
        || has("VOLUME")
        || has("NVME")
    {
        DeviceClass::Storage
    } else if has("CHIPSET")
        || has("SMBUS")
        || has("SM BUS")
        || has("HOSTBRIDGE")
        || has("HOST BRIDGE")
        || has("PCI BRIDGE")
    {
        DeviceClass::Chipset
    } else if has("SYSTEM")
        || has("PROCESSOR")
        || has("COMPUTER")
        || has("SOFTWARECOMPONENT")
        || has("SOFTWAREDEVICE")
        || has("EXTENSION")
    {
        DeviceClass::System
    } else {
        DeviceClass::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_classes() {
        assert_eq!(classify("MEDIA"), DeviceClass::Audio);
        assert_eq!(classify("AudioEndpoint"), DeviceClass::Audio);
        assert_eq!(classify("Display"), DeviceClass::Display);
        assert_eq!(classify("Monitor"), DeviceClass::Monitor);
        assert_eq!(classify("Net"), DeviceClass::Network);
        assert_eq!(classify("Bluetooth"), DeviceClass::Bluetooth);
        assert_eq!(classify("HIDClass"), DeviceClass::Input);
        assert_eq!(classify("Keyboard"), DeviceClass::Input);
        assert_eq!(classify("Mouse"), DeviceClass::Input);
        assert_eq!(classify("USB"), DeviceClass::Usb);
        assert_eq!(classify("DiskDrive"), DeviceClass::Storage);
        assert_eq!(classify("System"), DeviceClass::System);
        assert_eq!(classify("Processor"), DeviceClass::System);
        assert_eq!(classify("Printer"), DeviceClass::Printer);
        assert_eq!(classify("Image"), DeviceClass::Camera);
        assert_eq!(classify("Firmware"), DeviceClass::Firmware);
        assert_eq!(classify("Whatever"), DeviceClass::Other);
    }

    #[test]
    fn hid_keyboard_beats_system() {
        assert_eq!(classify("HIDClass"), DeviceClass::Input);
    }

    #[test]
    fn audio_before_media_ambiguity() {
        assert_eq!(classify("AUDIOENDPOINT"), DeviceClass::Audio);
        assert_eq!(classify("MEDIA"), DeviceClass::Audio);
    }

    #[test]
    fn classify_best_falls_back_to_title() {
        assert_eq!(
            classify_best("", "A-Volute AudioProcessingObject Driver Update (1.1.4.0)"),
            DeviceClass::Audio
        );
        assert_eq!(
            classify_best(
                "SoftwareComponent",
                "Nahimic SoftwareComponent Driver Update"
            ),
            DeviceClass::System
        );
        assert_eq!(
            classify_best("", "Dell Inc. - Monitor - 9/2/2015 12:00:00 AM - 1.0.0.0"),
            DeviceClass::Monitor
        );
        assert_eq!(
            classify_best("", "Realtek Driver Update (1.0.934.0) - MEDIA"),
            DeviceClass::Audio
        );
        assert_eq!(
            classify_best("", "HID-compliant vendor-defined device"),
            DeviceClass::Input
        );
        assert_eq!(
            classify_best("Net", "Some vague title"),
            DeviceClass::Network
        );
        assert_eq!(
            classify_best("", "Micro-Star INT'L CO., LTD. Driver Update"),
            DeviceClass::Other
        );
    }

    #[test]
    fn labels_are_nonempty() {
        for c in [
            DeviceClass::Audio,
            DeviceClass::Display,
            DeviceClass::Monitor,
            DeviceClass::Network,
            DeviceClass::Bluetooth,
            DeviceClass::Input,
            DeviceClass::Storage,
            DeviceClass::Printer,
            DeviceClass::Camera,
            DeviceClass::Sensor,
            DeviceClass::Battery,
            DeviceClass::SmartCard,
            DeviceClass::Firmware,
            DeviceClass::Chipset,
            DeviceClass::System,
            DeviceClass::Usb,
            DeviceClass::Other,
        ] {
            assert!(!c.label().is_empty());
        }
    }

    #[test]
    fn media_tuner_is_not_audio() {
        assert_eq!(classify("MEDIA"), DeviceClass::Audio);
        assert_eq!(
            classify_best("MEDIA", "Hauppauge WinTV TV Tuner"),
            DeviceClass::Camera
        );
        assert_eq!(
            classify_best("MEDIA", "Elgato Game Capture HD60"),
            DeviceClass::Camera
        );
    }

    #[test]
    fn monitor_splits_from_display() {
        assert_eq!(classify("Monitor"), DeviceClass::Monitor);
        assert_eq!(classify("Display"), DeviceClass::Display);
    }

    #[test]
    fn chipset_splits_from_system() {
        assert_eq!(classify("Chipset"), DeviceClass::Chipset);
        assert_eq!(classify("SMBUS Controller"), DeviceClass::Chipset);
        assert_eq!(classify("PCI Bridge"), DeviceClass::Chipset);
        assert_eq!(classify("System"), DeviceClass::System);
        assert_eq!(classify("SoftwareComponent"), DeviceClass::System);
    }

    #[test]
    fn sensor_battery_smartcard_classified() {
        assert_eq!(classify("Sensor"), DeviceClass::Sensor);
        assert_eq!(classify("Biometric"), DeviceClass::Sensor);
        assert_eq!(classify("Battery"), DeviceClass::Battery);
        assert_eq!(classify("SmartCardReader"), DeviceClass::SmartCard);
    }

    #[test]
    fn net_token_beats_usb() {
        assert_eq!(classify("USB\\Net wireless adapter"), DeviceClass::Network);
    }
}
