use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ClientSettings {
	pub disable_window_controls: bool,
	pub notification: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Methods {
	DBus,
	UnixSocket,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeneralSettings {
	pub communication_method: Methods,
	pub socket_path: Option<String>,
}

impl Default for GeneralSettings {
	fn default() -> Self {
		return Self {
			socket_path: Some("/run/user/$id/autoclicker.socket".into()),
			communication_method: Methods::DBus,
		};
	}
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct KeyboardSettings {
	pub disabled: bool,
	pub added_delay: u64,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MouseSettings {
	pub disabled: bool,
	pub added_delay: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DaemonSettings {
	pub hyprland_ipc: bool,
	pub dry_run: bool,

	pub mouse: MouseSettings,
	pub keyboard: KeyboardSettings,
}

impl Default for DaemonSettings {
	fn default() -> Self {
		return Self {
			dry_run: false,
			hyprland_ipc: true,
			keyboard: KeyboardSettings::default(),
			mouse: MouseSettings::default(),
		};
	}
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Settings {
	pub general: GeneralSettings,
	pub client: ClientSettings,
	pub daemon: DaemonSettings,
}
