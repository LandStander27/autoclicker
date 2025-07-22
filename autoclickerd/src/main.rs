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
use std::sync::mpsc::{self, Sender, Receiver};
use anyhow::{anyhow, Context};
use clap::Parser;
use evdev_rs::enums::EV_KEY;

#[allow(unused)]
use tracing::{debug, warn, error, info, trace, Level};

#[derive(Parser, Debug)]
#[command(name = "autoclickerd", version = version::version)]
#[command(about = "Daemon for dev.land.Autoclicker", long_about = None)]
struct Args {
	#[arg(short, long, help = "increase verbosity")]
	verbose: bool,
}

mod vdevice;
mod vmouse;
mod vkeyboard;
use vmouse::*;
use vkeyboard::*;
use common::prelude::*;

#[inline]
fn sleepms(ms: u64) {
	std::thread::sleep(std::time::Duration::from_millis(ms));
}

fn handle_stream(mut stream: &UnixStream, tx: &Sender<Message>) -> anyhow::Result<()> {
	let mut msg = String::new();
	stream.read_to_string(&mut msg).context("failed to read stream")?;

	let req = Message::decode(msg)?;
	trace!(?req);
	
	match req {
		Message::RepeatingMouseClick(ref event) => {
			if !["left", "right", "middle"].contains(&event.button.as_str()) {
				warn!("invalid mouse button");
				return Err(anyhow!("invalid mouse button"));
			}

			if !["single", "double"].contains(&event.typ.as_str()) {
				warn!("invalid click type");
				return Err(anyhow!("invalid click type"));
			}
		}
		Message::RepeatingKeyboardClick(_) => {}
		Message::StopClicking(_) => {}
		_ => {
			warn!("invalid request: {req:?}");
			return Err(anyhow!("invalid request"));
		}
	}
	
	tx.send(req).context("could not send event over channel")?;
	return Ok(());
}

fn socket_file() -> String {
	let id = nix::unistd::geteuid();
	return format!("/run/user/{id}/autoclicker.socket");
}

fn do_mouse_click(btn: &String, mouse: &Mouse) -> anyhow::Result<()> {
	if btn == "left" {
		mouse.click_mouse_button(MouseButton::Left).context("could not click button").unwrap();
	} else if btn == "right" {
		mouse.click_mouse_button(MouseButton::Right).context("could not click button").unwrap();
	} else if btn == "middle" {
		mouse.click_mouse_button(MouseButton::Middle).context("could not click button").unwrap();
	}
	
	return Ok(());
}

fn bg_thread(exiting: Arc<AtomicBool>, rx: Receiver<Message>, mouse: Mouse, keyboard: Keyboard) -> anyhow::Result<()> {
	let mut last_message = Message::StopClicking(StopClicking {});
	let mut last_click = std::time::Instant::now();
	let mut amount_clicked: u128 = 0;

	let mut parsed_keys: Vec<Vec<EV_KEY>> = Vec::new();
	let mut current_key: usize = 0;
	let mut last_repeat = std::time::Instant::now();
	let mut is_holding: bool = false;

	let timeout = std::time::Duration::from_millis(5);
	'outer: loop {
		if exiting.load(Ordering::Relaxed) {
			break;
		}

		match rx.recv_timeout(timeout) {
			Ok(msg) => {
				trace!("got msg from channel");
				last_click = std::time::Instant::now();
				amount_clicked = 0;
				last_message = msg;
				if let Message::RepeatingKeyboardClick(ref event) = last_message {
					parsed_keys.clear();
					for keys in &event.button {
						let mut outer = Vec::new();
						for key in keys {
							let ev_key: EV_KEY = match key.parse() {
								Ok(o) => o,
								Err(_) => {
									warn!("invalid keycode");
									continue 'outer;
								},
							};
							outer.push(ev_key);
						}
						parsed_keys.push(outer);
					}
					current_key = 0;
				}
			}

			Err(e) => {
				if e != mpsc::RecvTimeoutError::Timeout {
					panic!("recv_error: {e}");
				}
			}
		}

		match last_message {
			Message::StopClicking(_) => {}
			Message::RepeatingMouseClick(ref click) => {
				if click.amount != 0 && amount_clicked >= click.amount as u128 {
					continue;
				}
				
				if last_click.elapsed().as_millis() >= click.interval as u128 {
					last_click = std::time::Instant::now();
					mouse.move_mouse(click.position.0, click.position.1)?;
					do_mouse_click(&click.button, &mouse)?;
					if click.typ == "double" {
						std::thread::sleep(std::time::Duration::from_millis(50));
						do_mouse_click(&click.button, &mouse)?;
					}
					
					amount_clicked += 1;
				}
			}
			Message::RepeatingKeyboardClick(ref click) => {
				if click.amount != 0 && amount_clicked >= click.amount as u128 {
					continue;
				}
				
				if current_key == 0 && last_repeat.elapsed().as_millis() < click.delay_before_repeat as u128 {
					continue;
				}

				if is_holding && last_click.elapsed().as_millis() >= click.hold_duration as u128 {
					for key in parsed_keys[current_key].iter().rev() {
						keyboard.release_keyboard_button(*key)?;
					}
					last_repeat = std::time::Instant::now();
					current_key += 1;
					if current_key == parsed_keys.len() {
						amount_clicked += 1;
						current_key = 0;
					}
					is_holding = false;
				} else if is_holding {
					continue;
				}

				if last_click.elapsed().as_millis() >= click.interval as u128 {
					last_click = std::time::Instant::now();

					for key in &parsed_keys[current_key] {
						keyboard.press_keyboard_button(*key)?;
					}
					is_holding = true;
				}
			}
			_ => todo!()
		}
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
	let mouse = Mouse::new().context("could not create virtual mouse")?;

	trace!("creating virtual keyboard");
	let keyboard = Keyboard::new().context("could not create virtual keyboard")?;
	
	trace!("creating socket");
	let listener = UnixListener::bind(socket_file()).context("could not create socket")?;
	listener.set_nonblocking(true).context("could not set socket as nonblocking")?;
	trace!("binded");
	info!("listening");

	let (tx, rx) = mpsc::channel::<Message>();
	let exiting = Arc::new(AtomicBool::new(false));
	let clone = exiting.clone();
	let thread = std::thread::spawn(move || {
		if let Err(e) = bg_thread(clone, rx, mouse, keyboard) {
			error!("from bg_thread: {e}");
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
					sleepms(25);
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
