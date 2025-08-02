use anyhow::Context;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::mpsc::Sender;
use tracing::{info, trace};
use zbus::{connection, interface};

use common::prelude::*;

struct Handler<F, O>
where
	F: Fn(String) -> O + Send + Sync + 'static,
	O: Future<Output = anyhow::Result<Message>> + Send + 'static,
{
	tx: Sender<Message>,
	handler: Arc<F>,
}

impl<F, O> Handler<F, O>
where
	F: Fn(String) -> O + Send + Sync + 'static,
	O: Future<Output = anyhow::Result<Message>> + Send + 'static,
{
	async fn func(&self, msg: &str) -> anyhow::Result<String> {
		match (self.handler)(msg.to_string()).await {
			Ok(o) => {
				self.tx
					.send(o)
					.await
					.context("could not send event over channel")?;
				let message = Message::ConfirmResponse(ConfirmResponse {});
				let json = Message::encode(&message)?;
				return Ok(json);
			}
			Err(e) => {
				let message = Message::Error(ErrorResponse { msg: e.to_string() });

				let json = Message::encode(&message)?;
				return Ok(json);
			}
		}
	}
}

#[interface(name = "dev.land.Autoclicker1")]
impl<F, O> Handler<F, O>
where
	F: Fn(String) -> O + Send + Sync + 'static,
	O: Future<Output = anyhow::Result<Message>> + Send + 'static,
{
	async fn request(&self, msg: &str) -> String {
		return match self.func(msg).await {
			Ok(o) => o,
			Err(e) => format!("internal error: {e}"),
		};
	}
}

pub async fn listen<F, O>(tx: Sender<Message>, handle_msg: Arc<F>) -> anyhow::Result<()>
where
	F: Fn(String) -> O + Send + Sync + 'static,
	O: Future<Output = anyhow::Result<Message>> + Send + 'static,
{
	trace!("registering signal hooks");
	let mut int = signal(SignalKind::interrupt()).unwrap();
	let mut hup = signal(SignalKind::hangup()).unwrap();

	let handler = Handler { tx, handler: handle_msg };
	let _conn = connection::Builder::session()?
		.name("dev.land.Autoclicker")?
		.serve_at("/dev/land/Autoclicker", handler)?
		.build()
		.await?;

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
			_ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
		}
	}

	return Ok(());
}
