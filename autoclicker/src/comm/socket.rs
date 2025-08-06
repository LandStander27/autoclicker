use anyhow::Context;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use std::{
	io::{Read, Write},
	os::unix::net::UnixStream,
};
use tracing::error;

use crate::window::{dialogs, settings};
use common::prelude::*;

fn socket_file() -> String {
	let id = nix::unistd::geteuid();
	let arc = settings();
	let settings = arc.lock().unwrap();
	let path = &settings.general.socket_path;
	if path.is_none() {
		error!("socket_path must be supplied when communication_method = UnixSocket");
		std::process::exit(1);
	}
	return path
		.as_ref()
		.unwrap()
		.replace("$id", id.to_string().as_str());
}

pub(super) struct UnixSocket;
impl super::Method for UnixSocket {
	fn send_message(msg: &Message) -> anyhow::Result<()> {
		let mut stream = UnixStream::connect(socket_file()).context("could not connect to socket")?;
		let json = Message::encode(msg).context("could not encode as json")?;
		stream
			.write(json.as_bytes())
			.context("could not write to socket")?;

		stream
			.shutdown(std::net::Shutdown::Write)
			.context("could not shutdown writing")?;
		let mut msg = String::new();
		stream
			.read_to_string(&mut msg)
			.context("could not read from socket")?;
		let response = Message::decode(msg).context("could not decode json")?;

		if let Message::Error(e) = response {
			return Err(anyhow::anyhow!(e.msg));
		}

		return Ok(());
	}

	fn status(window: &ApplicationWindow) -> anyhow::Result<bool> {
		let s = socket_file();
		let file = std::path::Path::new(&s);
		if !file.exists() {
			tracing::debug!("spawning systemd service dialog");
			glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
			return Ok(false);
		}

		return Ok(true);
	}
}
