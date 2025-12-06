use anyhow::anyhow;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;

use crate::unix;
use crate::window::{ClickType, Config, MouseButton, Screen, dialogs, settings};
use common::prelude::*;

#[cfg(feature = "dbus")]
mod dbus;

#[cfg(feature = "socket")]
mod socket;

pub(super) trait Method {
	fn status(window: &ApplicationWindow) -> anyhow::Result<bool>;
	fn send_message(msg: &Message) -> anyhow::Result<()>;
}

pub fn stop(window: &ApplicationWindow) -> anyhow::Result<()> {
	if let Err(e) = send_message(&Message::StopClicking(StopClicking {})) {
		glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: comm::stop", e.to_string()));
		return Err(anyhow::anyhow!(e));
	}

	return Ok(());
}

pub fn start(window: &ApplicationWindow, config: &Config) -> anyhow::Result<()> {
	if !is_ready_to_start(window) {
		return Err(anyhow::anyhow!("daemon not ready"));
	}

	let message = match config.screen {
		Screen::Mouse => {
			let config = &config.mouse;
			Message::RepeatingMouseClick(RepeatingMouseClick {
				button: match config.mouse_button {
					MouseButton::Left => "left",
					MouseButton::Right => "right",
					MouseButton::Middle => "middle",
				}
				.to_string(),
				typ: match config.typ {
					ClickType::Single => "single",
					ClickType::Double => "double",
				}
				.to_string(),
				amount: config.repeat,
				interval: config.interval,
				position: (
					if config.enabled_axis.0 {
						Some(config.position.0)
					} else {
						None
					},
					if config.enabled_axis.1 {
						Some(config.position.1)
					} else {
						None
					},
				),
			})
		}
		Screen::Keyboard => {
			let config = &config.keyboard;
			let mut seq = config.sequence.clone();
			if config.enter_after {
				seq.extend([Actions::Press("KEY_ENTER".into()), Actions::Release("KEY_ENTER".into())]);
			}

			Message::RepeatingKeyboardClick(RepeatingKeyboardClick {
				buttons: seq,
				amount: config.repeat,
				interval: config.interval,
				delay_before_repeat: config.delay_before_repeat,
				hold_duration: config.hold_duration,
			})
		}
	};

	if let Err(e) = send_message(&message) {
		glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: comm::send_message", e.to_string()));
		return Err(anyhow!(e));
	}
	return Ok(());
}

fn send_message(msg: &Message) -> anyhow::Result<()> {
	if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
		#[cfg(feature = "dbus")]
		dbus::DBus::send_message(msg)?;

		#[cfg(not(feature = "dbus"))]
		{
			tracing::error!("this build was not compiled with dbus support");
			return Err(anyhow!("this build was not compiled with dbus support"));
		}
	} else {
		#[cfg(feature = "socket")]
		socket::UnixSocket::send_message(msg)?;

		#[cfg(not(feature = "socket"))]
		{
			tracing::error!("this build was not compiled with unix socket support");
			return Err(anyhow!("this build was not compiled with unix socket support"));
		}
	}

	#[allow(unreachable_code)]
	return Ok(());
}

pub fn is_ready_to_start(window: &ApplicationWindow) -> bool {
	let status: bool = if settings().lock().unwrap().general.communication_method == common::settings::latest::Methods::DBus {
		#[cfg(feature = "dbus")]
		match dbus::DBus::status(window) {
			Ok(o) => o,
			Err(e) => {
				glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: dbus::status", e.to_string()));
				false
			}
		}

		#[cfg(not(feature = "dbus"))]
		{
			tracing::error!("this build was not compiled with dbus support");
			glib::MainContext::default().spawn_local(dialogs::error_dialog(
				window.clone(),
				"Error: feature missing",
				"this build was not compiled with dbus support".into(),
			));
			false
		}
	} else {
		#[cfg(feature = "socket")]
		match socket::UnixSocket::status(window) {
			Ok(o) => o,
			Err(e) => {
				glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::status", e.to_string()));
				false
			}
		}

		#[cfg(not(feature = "socket"))]
		{
			tracing::error!("this build was not compiled with unix socket support");
			glib::MainContext::default().spawn_local(dialogs::error_dialog(
				window.clone(),
				"Error: feature missing",
				"this build was not compiled with unix socket support".into(),
			));
			false
		}
	};

	#[allow(unreachable_code)]
	if !status {
		return false;
	}

	let in_input = match unix::is_user_in_group("input") {
		Ok(o) => o,
		Err(e) => {
			glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: unix::is_user_in_group", e.to_string()));
			return false;
		}
	};

	if !in_input {
		tracing::debug!("spawning group dialog");
		glib::MainContext::default().spawn_local(dialogs::group_dialog(window.clone()));
		return false;
	}

	return true;
}
