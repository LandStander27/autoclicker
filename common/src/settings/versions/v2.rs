use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ClientSettings {
	pub disable_window_controls: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeneralSettings {
	pub socket_path: String,
}

impl Default for GeneralSettings {
	fn default() -> Self {
		return Self {
			socket_path: "/run/user/$id/autoclicker.socket".into(),
		};
	}
}

mod daemon {
	use serde::{Deserialize, Serialize};

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
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Settings {
	pub general: GeneralSettings,
	pub client: ClientSettings,
	pub daemon: daemon::DaemonSettings,
}
