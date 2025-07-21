use anyhow::{anyhow, Context};
use gtk4::{
	self as gtk, Button
};
use gtk::prelude::*;
use gtk::{
	ApplicationWindow,
	glib,
};

use std::sync::{Arc, Mutex};

use super::{
	Config,
	Screen,
	dialogs,
};

use crate::socket;
use crate::unix;
use crate::keycodes;

pub async fn get_coords() -> anyhow::Result<(i32, i32)> {
	let output = tokio::process::Command::new("/usr/bin/slurp")
		.args(["-b", "#00000000", "-p", "-f", "%x %y"])
		.output().await.context("could not run '/usr/bin/slurp'")?;

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
	if let Err(e) = socket::send_stop() {
		gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_stop", e.to_string()));
		return;
	}

	button.remove_css_class("destructive-action");
	button.add_css_class("suggested-action");
	button.set_label("Start");
}

fn start_mouse(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	fn status(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> bool {
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

		let s = socket::socket_file();
		let file = std::path::Path::new(&s);
		if !file.exists() {
			tracing::debug!("spawning systemd service dialog");
			glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
			return false;
		}
		
		if let Err(e) = socket::send_mouse_request(&config.lock().unwrap().mouse) {
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

fn start_keyboard(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	fn status(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> bool {
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

		let s = socket::socket_file();
		let file = std::path::Path::new(&s);
		if !file.exists() {
			tracing::debug!("spawning systemd service dialog");
			glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
			return false;
		}
		
		if let Err(e) = socket::send_keyboard_request(&config.lock().unwrap().keyboard) {
			gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_keybord_request", e.to_string()));
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

pub fn syntax_highlighting(debounce_id: &Arc<Mutex<Option<glib::SourceId>>>, buffer: gtk::TextBuffer) {
	let mut lock = debounce_id.lock().unwrap();
	if lock.is_some() {
		lock.take().unwrap().remove();
	}
	drop(lock);

	let clone = debounce_id.clone();
	let id = glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
		buffer.remove_all_tags(&buffer.start_iter(), &buffer.end_iter());
		let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).to_string();
		let input: Vec<char> = text.chars().collect();
		let mut i = 0;
		'outer: while i < input.len() {
			while input[i].is_whitespace() {
				i += 1;
				if input.len() - 1 <= i {
					break 'outer;
				}
			}

			if input[i] == '"' {
				let start = i;
				if input.len() - 1 <= i {
					break 'outer;
				}
				i += 1;
				while input[i] != '"' {
					i += 1;
					if i == input.len() {
						break;
					}
				}
				i += 1;
				let mut iter = buffer.start_iter();
				iter.forward_chars(start as i32);
				let mut end_iter = iter;
				end_iter.forward_chars((i - start) as i32);
				buffer.apply_tag_by_name("string", &iter, &end_iter);
				continue;
			}
			
			let mut token = String::new();
			while i < input.len() && !input[i].is_whitespace() {
				token.push(input[i]);
				i += 1;
			}

			let mut iter = buffer.start_iter();
			iter.forward_chars((i - token.len()) as i32);
			let mut end_iter = iter;
			end_iter.forward_chars(i as i32);
			if keycodes::key_exists(token.as_str()) {
				buffer.apply_tag_by_name("keycode", &iter, &end_iter);
			} else {
				buffer.apply_tag_by_name("invalid_keycode", &iter, &end_iter);
			}
		}
		*clone.lock().unwrap() = None;
		return glib::ControlFlow::Break;
	});
	*debounce_id.lock().unwrap() = Some(id);
}

pub fn parse_sequence(input: String) -> anyhow::Result<Vec<String>> {
	let mut parsed = Vec::new();

	let input: Vec<char> = input.chars().collect();
	let mut i = 0;
	while i < input.len() {
		while input[i].is_whitespace() {
			i += 1;
			if input.len() - 1 <= i {
				return Ok(parsed);
			}
		}

		if input[i] == '"' {
			if input.len() - 1 <= i {
				return Err(anyhow!("mismatched quotes"));
			}
			i += 1;
			while input[i] != '"' {
				if input[i] == ' ' {
					parsed.push("KEY_SPACE".into());
				} else if keycodes::key_exists(input[i].to_string().as_str()) {
					let mut s = String::new();
					s.push_str("KEY_");
					s.push_str(input[i].to_uppercase().to_string().as_str());
					parsed.push(s);
				} else {
					return Err(anyhow!("invalid char in quotes"));
				}
				i += 1;
				if i == input.len() {
					return Err(anyhow!("mismatched quotes"));
				}
			}
			i += 1;
			continue;
		}
		
		let mut token = String::new();
		while i < input.len() && !input[i].is_whitespace() {
			token.push(input[i]);
			i += 1;
		}
		if keycodes::key_exists(token.as_str()) {
			let mut s = String::new();
			s.push_str("KEY_");
			s.push_str(token.to_uppercase().as_str());
			parsed.push(s);
		} else {
			return Err(anyhow!("invalid key"));
		}
	}

	return Ok(parsed);
}
