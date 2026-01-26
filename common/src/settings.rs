use anyhow::Context;

mod versions;
pub use versions::*;

pub type Settings = v5::Settings;
pub use v5 as latest;

macro_rules! generate_trait {
	($($version:tt),* $(,)?) => {
		trait Latest {
			$(
				fn $version(old: $version::Settings) -> Self;
			)*
		}
	};
}

macro_rules! generate_froms {
	($($version:tt),* $(,)?) => {
		$(
			impl From<$version::Settings> for Settings {
				fn from(old: $version::Settings) -> Self {
					println!("from: {}", stringify!($version));
					return Self::$version(old);
				}
			}
		)*
	};
}

macro_rules! generate_migration {
	($path:ident, $($version:tt),*, $(,)?) => {
		$(
			if let Ok(o) = confy::load_or_else::<$version::Settings, _>($path.clone(), $version::Settings::default) {
				return Settings::from(o);
			}
		)*

		return Settings::default();
	};
}

macro_rules! generate_whole {
	($($version:tt),*) => {
		generate_trait!($($version,)*);
		generate_froms!($($version,)*);

		pub fn load() -> anyhow::Result<Settings> {
			let path = confy::get_configuration_file_path("dev.land.Autoclicker", Some("config")).context("could not get config file path")?;
			let config: Settings = confy::load_or_else(path.clone(), move || {
				generate_migration!(path, $($version, )*);
			}).context("could not load config file")?;

			return Ok(config);
		}
	};
}

generate_whole!(v4, v3, v2, v1);

impl Latest for Settings {
	fn v1(old: v1::Settings) -> Self {
		return Self {
			client: latest::ClientSettings {
				disable_window_controls: old.disable_window_controls,
				..Default::default()
			},
			..Default::default()
		};
	}

	fn v2(old: v2::Settings) -> Self {
		return Self {
			general: latest::GeneralSettings {
				socket_path: Some(old.general.socket_path),
				communication_method: latest::Methods::UnixSocket,
			},
			client: latest::ClientSettings {
				disable_window_controls: old.client.disable_window_controls,
				..Default::default()
			},
			daemon: latest::DaemonSettings {
				hyprland_ipc: old.daemon.hyprland_ipc,
				dry_run: old.daemon.dry_run,
				mouse: latest::MouseSettings {
					added_delay: old.daemon.mouse.added_delay,
					disabled: old.daemon.mouse.disabled,
				},
				keyboard: latest::KeyboardSettings {
					added_delay: old.daemon.keyboard.added_delay,
					disabled: old.daemon.keyboard.disabled,
				},
			},
		};
	}

	fn v3(old: v3::Settings) -> Self {
		return Self {
			general: latest::GeneralSettings {
				socket_path: old.general.socket_path,
				communication_method: if old.general.communication_method == v3::Methods::DBus {
					latest::Methods::DBus
				} else {
					latest::Methods::UnixSocket
				},
			},
			client: latest::ClientSettings {
				disable_window_controls: old.client.disable_window_controls,
				..Default::default()
			},
			daemon: latest::DaemonSettings {
				hyprland_ipc: old.daemon.hyprland_ipc,
				dry_run: old.daemon.dry_run,
				mouse: latest::MouseSettings {
					added_delay: old.daemon.mouse.added_delay,
					disabled: old.daemon.mouse.disabled,
				},
				keyboard: latest::KeyboardSettings {
					added_delay: old.daemon.keyboard.added_delay,
					disabled: old.daemon.keyboard.disabled,
				},
			},
		};
	}

	fn v4(old: v4::Settings) -> Self {
		return Self {
			general: latest::GeneralSettings {
				socket_path: old.general.socket_path,
				communication_method: if old.general.communication_method == v4::Methods::DBus {
					latest::Methods::DBus
				} else {
					latest::Methods::UnixSocket
				},
			},
			client: latest::ClientSettings {
				disable_window_controls: old.client.disable_window_controls,
				notification: old.client.notification,
				..Default::default()
			},
			daemon: latest::DaemonSettings {
				hyprland_ipc: old.daemon.hyprland_ipc,
				dry_run: old.daemon.dry_run,
				mouse: latest::MouseSettings {
					added_delay: old.daemon.mouse.added_delay,
					disabled: old.daemon.mouse.disabled,
				},
				keyboard: latest::KeyboardSettings {
					added_delay: old.daemon.keyboard.added_delay,
					disabled: old.daemon.keyboard.disabled,
				},
			},
		};
	}
}

pub fn save(settings: &Settings) -> anyhow::Result<()> {
	return confy::store("dev.land.Autoclicker", Some("config"), settings).context("could not store config");
}
