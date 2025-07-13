use std::{io::{Read, Write}, os::unix::net::{UnixListener, UnixStream}, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use anyhow::{anyhow, Context};
use clap::Parser;

#[allow(unused)]
use tracing::{debug, warn, error, info, trace, Level};

#[derive(Parser, Debug)]
#[command(name = "autoclickerd", version = version::version)]
#[command(about = "Daemon for autoclicker", long_about = None)]
struct Args {
	#[arg(short, long, help = "increase verbosity")]
	verbose: bool,
}

mod vmouse;
use vmouse::*;
use common::prelude::*;

fn handle_stream(mut stream: &UnixStream, mouse: &Mouse) -> anyhow::Result<()> {
	let mut msg = String::new();
	stream.read_to_string(&mut msg).context("failed to read stream")?;

	let req = Message::decode(msg)?;
	trace!(?req);
	match req {
		Message::MouseClick(event) => {
			if event.button == "left" {
				info!("left mouse button");
				mouse.click_button(MouseButton::Left).context("failed to execute event")?;
			} else if event.button == "right" {
				info!("right mouse button");
				mouse.click_button(MouseButton::Right).context("failed to execute event")?;
			} else {
				warn!("invalid mouse button");
				return Err(anyhow!("invalid mouse button"));
			}
		}

		Message::Error(_) => {
			warn!("invalid request");
		}
	}

	return Ok(());
}

fn socket_file() -> String {
	let id = nix::unistd::geteuid();
	return format!("/run/user/{}/autoclicker.socket", id);
}

fn main() -> anyhow::Result<()> {
	let args = Args::parse();
	
	let subscriber = tracing_subscriber::fmt()
		.compact()
		.with_file(false)
		.with_line_number(false)
		.with_thread_ids(false)
		.with_target(true)
		.with_max_level(if args.verbose { Level::TRACE } else { Level::DEBUG })
		.without_time()
		.finish();
	tracing::subscriber::set_global_default(subscriber).unwrap();
	
	trace!("registered logger");
	trace!("registering signal hooks");
	let hup = Arc::new(AtomicBool::new(false));
	let int = Arc::new(AtomicBool::new(false));
	signal_hook::flag::register(signal_hook::consts::SIGHUP, hup.clone()).context("could not register SIGHUP hook")?;
	signal_hook::flag::register(signal_hook::consts::SIGINT, int.clone()).context("could not register SIGINT hook")?;
	
	trace!("creating virtual mouse");
	let mouse = Mouse::new().context("could not create virtual device")?;
	
	trace!("creating socket");
	let listener = UnixListener::bind(socket_file()).context("could not create socket")?;
	listener.set_nonblocking(true).context("could not set socket as nonblocking")?;
	trace!("binded");
	info!("listening");
	
	loop {
		if hup.load(Ordering::Relaxed) || int.load(Ordering::Relaxed) {
			if int.load(Ordering::Relaxed) {
				println!();
			}
			info!("gracefully shutting down");
			break;
		}

		let res = listener.accept();
		match res {
			Ok((mut stream, _addr)) => {
				if let Err(e) = handle_stream(&stream, &mouse) {
					let message = Message::Error(ErrorResponse {
						msg: e.to_string(),
					});
					
					let json = Message::encode(&message)?;
					stream.write(json.as_bytes()).context("could not write to stream")?;
				}

			}
			Err(ref e) => {
				if e.kind() == std::io::ErrorKind::WouldBlock {
					continue;
				}
				res.context("failed at accepting connection on socket")?;
			}
		}
	}

	drop(listener);
	std::fs::remove_file(socket_file()).context("could not delete socket")?;
	trace!("deleted socket");
	
	return Ok(());
}
