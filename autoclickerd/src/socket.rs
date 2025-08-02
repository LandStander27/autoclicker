use super::settings;
use anyhow::Context;
use std::sync::Arc;
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::{UnixListener, UnixStream},
	signal::unix::{SignalKind, signal},
	sync::mpsc::Sender,
};
use tracing::{error, info, trace};

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

pub async fn listen<F, O>(tx: Sender<Message>, handle_msg: Arc<F>) -> anyhow::Result<()>
where
	F: Fn(String) -> O + Send + Sync + 'static,
	O: Future<Output = anyhow::Result<Message>> + Send,
{
	trace!("registering signal hooks");
	let mut int = signal(SignalKind::interrupt()).unwrap();
	let mut hup = signal(SignalKind::hangup()).unwrap();

	trace!("creating socket");
	let listener = {
		let path_str = socket_file();
		let path = std::path::Path::new(&path_str);
		std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new("")))?;
		UnixListener::bind(path).context("could not create socket")?
	};
	trace!("binded");

	let func = async |mut stream: UnixStream, tx: Sender<Message>, handler: Arc<F>| -> anyhow::Result<()> {
		let mut msg = String::new();
		stream
			.read_to_string(&mut msg)
			.await
			.context("failed to read stream")?;

		match handler(msg).await {
			Ok(o) => {
				tx.send(o)
					.await
					.context("could not send event over channel")?;
				let message = Message::ConfirmResponse(ConfirmResponse {});
				let json = Message::encode(&message)?;
				stream
					.write_all(json.as_bytes())
					.await
					.context("could not write to stream")?;
			}
			Err(e) => {
				let message = Message::Error(ErrorResponse { msg: e.to_string() });

				let json = Message::encode(&message)?;
				stream
					.write_all(json.as_bytes())
					.await
					.context("could not write to stream")?;
			}
		}

		return Ok(());
	};

	info!("listening");
	loop {
		let int = int.recv();
		let hup = hup.recv();
		tokio::select! {
			_ = int => {
				println!();
				info!("gracefully shutting down");
				break;
			}
			_ = hup => {
				info!("gracefully shutting down");
				break;
			}
			Ok((stream, _)) = listener.accept() => {
				let tx = tx.clone();
				tokio::spawn(func(stream, tx, handle_msg.clone()));
			}
		}
	}

	drop(listener);
	std::fs::remove_file(socket_file()).context("could not delete socket")?;
	trace!("deleted socket");

	return Ok(());
}
