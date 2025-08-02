use anyhow::Context;
use tokio::sync::OnceCell;
use zbus::{Connection, proxy};

use super::window::{KeyboardConfig, MouseConfig};
use crate::{ClickType, MouseButton};
use common::prelude::*;

#[proxy(interface = "dev.land.Autoclicker1", default_service = "dev.land.Autoclicker", default_path = "/dev/land/Autoclicker")]
trait Daemon {
	fn request(&self, msg: &str) -> zbus::Result<String>;
}

async fn proxy() -> anyhow::Result<&'static DaemonProxy<'static>> {
	static PROXY: OnceCell<DaemonProxy> = OnceCell::const_new();
	if PROXY.get().is_none() {
		let connection = Connection::session()
			.await
			.context("could not start dbus session")?;
		// {
		// 	let a = zbus::fdo::DBusProxy::new(&connection).await?;
		// 	if !a
		// 		.name_has_owner("dev.land.Autoclicker".try_into().unwrap())
		// 		.await?
		// 	{}
		// }
		let proxy = DaemonProxy::new(&connection)
			.await
			.context("could not create dbus proxy")?;
		return Ok(PROXY.get_or_init(async move || proxy).await);
	}
	return Ok(PROXY.get().as_ref().unwrap());
}

async fn send(msg: &str) -> anyhow::Result<String> {
	let proxy = proxy().await?;
	return proxy.request(msg).await.context("could not send request");
}

pub async fn send_stop() -> anyhow::Result<()> {
	let request = Message::StopClicking(StopClicking {});
	let json = Message::encode(&request).context("could not encode as json")?;
	let response = Message::decode(send(&json).await?).context("could not decode json")?;

	if let Message::Error(e) = response {
		return Err(anyhow::anyhow!(e.msg));
	}

	return Ok(());
}

pub async fn send_mouse_request(config: &MouseConfig) -> anyhow::Result<()> {
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
	});

	let json = Message::encode(&request).context("could not encode as json")?;
	let response = Message::decode(send(&json).await?).context("could not decode json")?;

	if let Message::Error(e) = response {
		return Err(anyhow::anyhow!(e.msg));
	}

	return Ok(());
}

pub async fn send_keyboard_request(config: &KeyboardConfig) -> anyhow::Result<()> {
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
	let response = Message::decode(send(&json).await?).context("could not decode json")?;

	if let Message::Error(e) = response {
		return Err(anyhow::anyhow!(e.msg));
	}

	return Ok(());
}
