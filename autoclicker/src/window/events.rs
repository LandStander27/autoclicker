use anyhow::{Context, anyhow};
use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4::{self as gtk, Button};

use std::sync::{Arc, Mutex};

use super::Config;

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

	#[allow(clippy::collapsible_if)]
	if s == "Start" {
		if crate::comm::start(window, &config.lock().unwrap()).is_ok() {
			button.remove_css_class("suggested-action");
			button.add_css_class("destructive-action");
			button.set_label("Stop");
		}
	} else if s == "Stop" {
		if crate::comm::stop(window).is_ok() {
			button.remove_css_class("destructive-action");
			button.add_css_class("suggested-action");
			button.set_label("Start");
		}
	}
}
