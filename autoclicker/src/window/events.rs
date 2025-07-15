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
	dialogs,
};

use crate::socket;

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
	if s == "Start" {
		start_clicking(window, button, config);
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

fn start_clicking(window: &ApplicationWindow, button: &Button, config: Arc<Mutex<Config>>) {
	let status = 'outer: {
		let groups = match nix::unistd::getgroups() {
			Ok(g) => g,
			Err(e) => {
				gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: getgroups", e.to_string()));
				break 'outer false;
			}
		};

		let mut in_input = false;
		for group in groups {
			let group = match nix::unistd::Group::from_gid(group) {
				Ok(g) => match g {
					Some(g) => g,
					None => {
						gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: from_gid", "group does not exist".to_string()));
						break 'outer false;
					}
				},
				Err(e) => {
					gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: from_gid", e.to_string()));
					break 'outer false;
				}
			};
			if group.name == "input" {
				in_input = true;
				break;
			}
		}
		if !in_input {
			tracing::debug!("spawning group dialog");
			glib::MainContext::default().spawn_local(dialogs::group_dialog(window.clone()));
			break 'outer false;
		}

		let s = socket::socket_file();
		let file = std::path::Path::new(&s);
		if !file.exists() {
			tracing::debug!("spawning systemd service dialog");
			glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
			break 'outer false;
		}
		
		if let Err(e) = socket::send_request(config) {
			gtk::glib::MainContext::default().spawn_local(dialogs::error_dialog(window.clone(), "Error: socket::send_request", e.to_string()));
			break 'outer false;
		}
		
		true
	};
	
	if !status {
		return;
	}

	button.remove_css_class("suggested-action");
	button.add_css_class("destructive-action");
	button.set_label("Stop");
}