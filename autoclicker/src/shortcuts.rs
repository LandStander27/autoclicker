#![allow(non_upper_case_globals)]

use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use ashpd::{
	desktop::{
		Session,
		global_shortcuts::{
			Activated,
			GlobalShortcuts,
			NewShortcut,
		},
		ResponseError
	},
	Error::Response,
	WindowIdentifier
};

use futures_util::{stream, StreamExt};
use gtk4 as gtk;
use gtk::prelude::*;

static global_shortcuts: OnceLock<GlobalShortcuts<'_>> = OnceLock::new();
static global_session: OnceLock<Session<GlobalShortcuts<'_>>> = OnceLock::new();

pub async fn stop_session() -> anyhow::Result<()> {
	global_session.get().context("session not inited")?.close().await.context("could not close session")?;
	return Ok(());
}

pub async fn listen_events<F: Fn()>(func: F) -> anyhow::Result<()> {
	let shortcuts = global_shortcuts.get().context("session not inited")?;
	let Ok(activated_stream) = shortcuts.receive_activated().await else {
		return Err(anyhow!("could not receive activated shortcuts"));
	};
	
	enum Event {
		Activated(Activated),
	}
	
	let bact: Box<dyn stream::Stream<Item = Event> + Unpin> = Box::new(activated_stream.map(Event::Activated));
	
	let mut events = stream::select_all([bact]);
	while let Some(event) = events.next().await {
		let Event::Activated(activation) = event;
		tracing::debug!(?activation);
		func();
	}
	
	return Ok(());
}

pub async fn start_session<W: IsA<gtk::Widget>>(widget: &W) -> anyhow::Result<()> {
	let root = widget.native().unwrap();
	let ident = WindowIdentifier::from_native(&root).await;
	let shortcut = NewShortcut::new("toggle-clicking", "Toggle clicking").preferred_trigger("F6");
	
	let shortcuts = GlobalShortcuts::new().await.context("could not get GlobalShortcuts portal")?;
	let session = shortcuts.create_session().await.context("could not create GlobalShortcuts session")?;
	let shortcuts_vec = [ shortcut ];
	let request = shortcuts.bind_shortcuts(&session, &shortcuts_vec, ident.as_ref()).await.context("could not bind shortcuts")?;
	let response = request.response();
	if let Err(e) = &response {
		return Err(match e {
			Response(ResponseError::Cancelled) => anyhow!("cancelled"),
			other => anyhow!("{}", other),
		});
	}
	global_shortcuts.set(shortcuts).unwrap();
	global_session.set(session).unwrap();
	
	return Ok(());
}
