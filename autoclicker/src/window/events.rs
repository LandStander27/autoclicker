use anyhow::{Context, anyhow};
use gtk::prelude::*;
use gtk::{ApplicationWindow, glib};
use gtk4::{self as gtk, Button};

use std::sync::{Arc, Mutex};

use super::{Config, Screen, dialogs};
use crate::unix;
use crate::window::runtime;
use crate::{dbus, settings, socket};

pub async fn get_coords() -> anyhow::Result<(i32, i32)> {
	let output = tokio::process::Command::new("/usr/bin/slurp")
		.args(["-b", "#00000000", "-p", "-f", "%x %y"])
		.output()
		.await
		.context("could not run '/usr/bin/slurp'")?;

	if !output.status.success() {
		return Err(anyhow!("slurp failed, code: {}", output.status));
	}

	let output = String::from_utf8_lossy(output.stdout.as_slice()).to_string();
	tracing::debug!(slurp_output = output);

	let pos: Vec<&str> = output.split(" ").collect();
	if pos.len() != 2 {
		return Err(anyhow!("invalid slurp output"));
	}

	let pos: (i32, i32) = (pos[0].parse().context("invalid slurp output")?, pos[1].parse().context("invalid slurp output")?);
	return Ok(pos);
}

pub fn primary_button(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	let s = button.label().unwrap();
	let screen = config.lock().unwrap().screen.clone();
	if s == "Start" {
		match screen {
			Screen::Mouse => start_mouse(window, button, config),
			Screen::Keyboard => start_keyboard(window, button, config),
		}
	} else if s == "Stop" {
		stop_clicking(window, button);
	}
}

fn stop_clicking(window: &ApplicationWindow, button: &Button) {
	if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
		runtime().block_on(async {
			if let Err(e) = dbus::send_stop().await {
				gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_stop", e.to_string()));
				return;
			}
		});
	} else if let Err(e) = socket::send_stop() {
		gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_stop", e.to_string()));
		return;
	}

	button.remove_css_class("destructive-action");
	button.add_css_class("suggested-action");
	button.set_label("Start");
}

fn start_mouse(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	fn status(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> bool {
		if !is_okay(window) {
			return false;
		}

		if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
			let config = config.lock().unwrap();
			let ret = runtime().block_on(async {
				if let Err(e) = dbus::send_mouse_request(&config.mouse).await {
					gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: dbus::send_keyboard_request", e.to_string()));
					return false;
				}

				return true;
			});

			return ret;
		} else if let Err(e) = socket::send_mouse_request(&config.lock().unwrap().mouse) {
			gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_keyboard_request", e.to_string()));
			return false;
		}

		return true;
	}

	if !status(window, config) {
		return;
	}

	button.remove_css_class("suggested-action");
	button.add_css_class("destructive-action");
	button.set_label("Stop");
}

fn start_keyboard(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	fn status(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> bool {
		if !is_okay(window) {
			return false;
		}

		if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
			let config = config.lock().unwrap();
			let ret = runtime().block_on(async {
				if let Err(e) = dbus::send_keyboard_request(&config.keyboard).await {
					gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: dbus::send_mouse_request", e.to_string()));
					return false;
				}

				return true;
			});

			return ret;
		} else if let Err(e) = socket::send_keyboard_request(&config.lock().unwrap().keyboard) {
			gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_mouse_request", e.to_string()));
			return false;
		}

		return true;
	}

	if !status(window, config) {
		return;
	}

	button.remove_css_class("suggested-action");
	button.add_css_class("destructive-action");
	button.set_label("Stop");
}

fn is_okay(window: &ApplicationWindow) -> bool {
	let in_input = match unix::is_user_in_group("input") {
		Ok(o) => o,
		Err(e) => {
			gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: unix::is_user_in_group", e.to_string()));
			return false;
		}
	};

	if !in_input {
		tracing::debug!("spawning group dialog");
		glib::MainContext::default().spawn_local(dialogs::group_dialog(window.clone()));
		return false;
	}

	if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
		let ret = runtime().block_on(async {
			let connection = zbus::Connection::session()
				.await
				.context("could not start dbus session")?;
			let proxy = zbus::fdo::DBusProxy::new(&connection)
				.await
				.context("could not start dbus proxy")?;
			if !proxy
				.name_has_owner("dev.land.Autoclicker".try_into().unwrap())
				.await
				.context("could not check if name has owner")?
			{
				tracing::debug!("spawning systemd service dialog");
				gtk::glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
				return Ok(false);
			}

			return Ok::<bool, anyhow::Error>(true);
		});
		match ret {
			Ok(o) => {
				if !o {
					return o;
				}
			}
			Err(e) => {
				gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: checking if service exists", e.to_string()));
				return false;
			}
		}

		return true;
	} else {
		let s = socket::socket_file();
		let file = std::path::Path::new(&s);
		if !file.exists() {
			tracing::debug!("spawning systemd service dialog");
			glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
			return false;
		}
	}

	return true;
}
