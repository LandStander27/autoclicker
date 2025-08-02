use std::{
	io::{Read, Write},
	os::unix::net::UnixStream,
};

use crate::vmouse::Mouse;
use anyhow::{Context, anyhow};

#[inline]
pub fn is_hyprland() -> bool {
	return std::env::var("XDG_CURRENT_DESKTOP").unwrap_or("".to_string()) == "Hyprland";
}

#[inline]
pub fn move_mouse(mouse: &Mouse, x: Option<i32>, y: Option<i32>) -> anyhow::Result<()> {
	debug_assert!(is_hyprland());

	mouse.move_mouse(x, y)?;
	let new_pos = get_pos().context("could not get current cursorpos")?;
	if (x.is_some() && new_pos.0 != x.unwrap_or(-99999)) || (y.is_some() && new_pos.1 != y.unwrap_or(-99999)) {
		let new_x = x.map(|x| x - new_pos.0);
		let new_y = y.map(|y| y - new_pos.1);

		mouse.move_mouse_relative(new_x, new_y)?;
	}

	return Ok(());
}

#[inline]
fn socket_file() -> anyhow::Result<String> {
	return Ok(format!("{}/hypr/{}/.socket.sock", std::env::var("XDG_RUNTIME_DIR")?, std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?));
}

#[inline]
fn get_pos() -> anyhow::Result<(i32, i32)> {
	let mut stream = UnixStream::connect(socket_file()?).context("could not connect to socket")?;
	stream
		.write(b"/cursorpos")
		.context("could not write to socket")?;

	stream
		.shutdown(std::net::Shutdown::Write)
		.context("could not shutdown writing")?;
	let mut msg = String::new();
	stream
		.read_to_string(&mut msg)
		.context("could not read from socket")?;

	let pos: Vec<&str> = msg.split(", ").collect();
	if pos.len() != 2 {
		return Err(anyhow!("invalid response from hyprctl"));
	}

	let x: i32 = pos[0].parse().context("invalid response from hyprctl")?;
	let y: i32 = pos[1].parse().context("invalid response from hyprctl")?;

	return Ok((x, y));
}
