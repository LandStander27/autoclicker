use std::{
	io::{Read, Write},
	os::unix::net::UnixStream,
};

use crate::{ClickType, MouseButton};
use anyhow::Context;

use super::window::{KeyboardConfig, MouseConfig};
use common::prelude::*;

pub fn socket_file() -> String {
	let id = nix::unistd::geteuid();
	let arc = crate::window::settings();
	let settings = arc.lock().unwrap();
	let path = &settings.general.socket_path;

	assert!(path.is_some(), "socket_path must be supplied when communication_method = UnixSocket");
	return path
		.as_ref()
		.unwrap()
		.replace("$id", id.to_string().as_str());
}

pub fn send_stop() -> anyhow::Result<()> {
	let mut stream = UnixStream::connect(socket_file()).context("could not connect to socket")?;
	let request = Message::StopClicking(StopClicking {});

	let json = Message::encode(&request).context("could not encode as json")?;
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

pub fn send_mouse_request(config: &MouseConfig) -> anyhow::Result<()> {
	let mut stream = UnixStream::connect(socket_file()).context("could not connect to socket")?;
	let request = Message::RepeatingMouseClick(RepeatingMouseClick {
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
		// delay_until_first_click: 2000,
	});

	let json = Message::encode(&request).context("could not encode as json")?;
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

pub fn send_keyboard_request(config: &KeyboardConfig) -> anyhow::Result<()> {
	let mut stream = UnixStream::connect(socket_file()).context("could not connect to socket")?;
	let mut seq = config.sequence.clone();
	if config.enter_after {
		seq.extend([Actions::Press("KEY_ENTER".into()), Actions::Release("KEY_ENTER".into())]);
	}

	let request = Message::RepeatingKeyboardClick(RepeatingKeyboardClick {
		buttons: seq,
		amount: config.repeat,
		interval: config.interval,
		delay_before_repeat: config.delay_before_repeat,
		hold_duration: config.hold_duration,
	});
	let json = Message::encode(&request).context("could not encode as json")?;
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
