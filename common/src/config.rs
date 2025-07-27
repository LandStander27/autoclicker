use serde::{Deserialize, Serialize};
use anyhow::Context;

#[derive(Default, Serialize, Deserialize)]
struct SettingsV1 {
	pub disable_window_controls: bool,
}

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
			socket_path: "/run/user/$id/autoclicker.socket".into()
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
pub struct SettingsV2 {
	pub general: GeneralSettings,
	pub client: ClientSettings,
	pub daemon: daemon::DaemonSettings,
}

impl From<SettingsV1> for SettingsV2 {
	fn from(old: SettingsV1) -> Self {
		return Self {
			client: ClientSettings {
				disable_window_controls: old.disable_window_controls,
			},
			..Default::default()
		};
	}
}

pub type Settings = SettingsV2;

pub fn load() -> anyhow::Result<Settings> {
	let path = confy::get_configuration_file_path("dev.land.Autoclicker", Some("config")).context("could not get config file path")?;
	let config: Settings = confy::load_or_else(path.clone(), move || {
		let old: SettingsV1 = match confy::load_or_else(path, || {
			return SettingsV1::default();
		}) {
			Ok(o) => o,
			Err(_e) => return Settings::default(),
		};

		return Settings::from(old);
	}).context("could not load config file")?;
	
	return Ok(config);
}

pub fn save(settings: &Settings) -> anyhow::Result<()> {
	return confy::store("dev.land.Autoclicker", Some("config"), settings).context("could not store config");
}
