use std::{
	io::{
		Read,
		Write,
	},
	os::unix::net::{
		UnixListener,
		UnixStream
	}, 
	sync::{
		atomic::{
			AtomicBool,
			Ordering
		},
		Arc
	}
};
use std::sync::mpsc::{self, Sender};

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
use common::{prelude::*, ConfirmResponse};

fn handle_stream(mut stream: &UnixStream, tx: &Sender<Option<RepeatingClick>>) -> anyhow::Result<()> {
	let mut msg = String::new();
	stream.read_to_string(&mut msg).context("failed to read stream")?;

	let req = Message::decode(msg)?;
	trace!(?req);
	match req {
		Message::MouseClick(_event) => {
			// if event.button == "left" {
			// 	info!("left mouse button");
			// 	mouse.lock().unwrap().click_button(MouseButton::Left).context("failed to execute event")?;
			// } else if event.button == "right" {
			// 	info!("right mouse button");
			// 	mouse.lock().unwrap().click_button(MouseButton::Right).context("failed to execute event")?;
			// } else {
			// 	warn!("invalid mouse button");
			// 	return Err(anyhow!("invalid mouse button"));
			// }
		}

		Message::RepeatingClick(event) => {
			if !["left", "right", "middle"].contains(&event.button.as_str()) {
				warn!("invalid mouse button");
				return Err(anyhow!("invalid mouse button"));
			}

			if !["single", "double"].contains(&event.typ.as_str()) {
				warn!("invalid click type");
				return Err(anyhow!("invalid click type"));
			}

			tx.send(Some(event)).context("could not send click over channel")?;
		}

		Message::StopClicking(_) => {
			tx.send(None).context("could not send click over channel")?;
		}

		_ => {
			warn!("invalid request");
		}
	}

	return Ok(());
}

fn socket_file() -> String {
	let id = nix::unistd::geteuid();
	return format!("/run/user/{}/autoclicker.socket", id);
}

fn do_click(btn: &String, mouse: &Mouse) -> anyhow::Result<()> {
	if btn == "left" {
		mouse.click_button(MouseButton::Left).context("could not click button").unwrap();
	} else if btn == "right" {
		mouse.click_button(MouseButton::Right).context("could not click button").unwrap();
	} else if btn == "middle" {
		mouse.click_button(MouseButton::Middle).context("could not click button").unwrap();
	}
	
	return Ok(());
}

fn main() -> anyhow::Result<()> {
	let args = Args::parse();
	
	let subscriber = tracing_subscriber::fmt()
		.compact()
		.with_file(false)
		.with_line_number(false)
		.with_thread_ids(true)
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

	let (tx, rx) = mpsc::channel::<Option<RepeatingClick>>();
	let exiting = Arc::new(AtomicBool::new(false));
	let clone = exiting.clone();
	let thread = std::thread::spawn(move || {
		let exiting = clone.clone();
		let mut click = None;
		let mut last_click = std::time::Instant::now();
		let mut first_click = false;
		let mut amount_clicked: u128 = 0;
		loop {
			if exiting.load(Ordering::Relaxed) {
				break;
			}

			match rx.try_recv() {
				Ok(value) => {
					trace!("got value from channel");
					last_click = std::time::Instant::now();
					amount_clicked = 0;
					click = value;
					first_click = click.is_some();
				}

				Err(e) => {
					if e != mpsc::TryRecvError::Empty {
						panic!("recv_error: {}", e);
					}
				}
			}

			if let Some(event) = &click {
				if first_click && last_click.elapsed().as_millis() < event.delay_until_first_click as u128 {
					continue;
				}

				if event.amount != 0 && amount_clicked >= event.amount as u128 {
					continue;
				}
				
				if last_click.elapsed().as_millis() >= event.interval as u128 {
					last_click = std::time::Instant::now();
					first_click = false;
					mouse.move_mouse(event.position.0, event.position.1).unwrap();
					do_click(&event.button, &mouse).unwrap();
					if event.typ == "double" {
						std::thread::sleep(std::time::Duration::from_millis(50));
						do_click(&event.button, &mouse).unwrap();
					}
					
					amount_clicked += 1;
				}
			}
		}
	});

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
				if let Err(e) = handle_stream(&stream, &tx) {
					let message = Message::Error(ErrorResponse {
						msg: e.to_string(),
					});
					
					let json = Message::encode(&message)?;
					stream.write(json.as_bytes()).context("could not write to stream")?;
				} else {
					let message = Message::ConfirmResponse(ConfirmResponse {});
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
	
	exiting.store(true, Ordering::Relaxed);
	thread.join().unwrap();
	
	return Ok(());
}
