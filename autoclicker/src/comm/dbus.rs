use anyhow::Context;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use tokio::sync::OnceCell;
use zbus::{Connection, proxy};

use crate::window::{dialogs, runtime};
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
		let proxy = DaemonProxy::new(&connection)
			.await
			.context("could not create dbus proxy")?;
		return Ok(PROXY.get_or_init(async move || proxy).await);
	}
	return Ok(PROXY.get().as_ref().unwrap());
}

pub(super) struct DBus;
impl super::Method for DBus {
	fn send_message(msg: &Message) -> anyhow::Result<()> {
		return crate::window::runtime().block_on(async {
			let proxy = proxy().await?;

			let json = Message::encode(msg).context("could not encode as json")?;
			let response = Message::decode(
				proxy
					.request(&json)
					.await
					.context("could not send request")?,
			)
			.context("could not decode json")?;

			if let Message::Error(e) = response {
				return Err(anyhow::anyhow!(e.msg));
			}

			return Ok(());
		});
	}

	fn status(window: &ApplicationWindow) -> anyhow::Result<bool> {
		return runtime().block_on(async {
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
				glib::MainContext::default().spawn_local(dialogs::service_dialog(window.clone()));
				return Ok(false);
			}

			return Ok(true);
		});
	}
}
