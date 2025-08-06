use anyhow::{Context, anyhow};
use clap::Parser;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{
	Notify,
	mpsc::{self, Receiver},
};

use evdev_rs::enums::EV_KEY;
#[allow(unused)]
use tracing::{Level, debug, error, info, trace, warn};

#[derive(Parser, Debug)]
#[command(name = "autoclickerd", version = version::version)]
#[command(about = "Daemon for dev.land.Autoclicker", long_about = None)]
struct Args {
	#[arg(short, long, help = "increase verbosity")]
	verbose: bool,

	#[arg(short, long, help = "do not actually take action on any request")]
	dry_run: bool,
}

#[cfg(not(any(feature = "socket", feature = "dbus")))]
compile_error!("either dbus or socket must be enabled");

#[cfg(feature = "socket")]
mod socket;

#[cfg(feature = "dbus")]
mod dbus;

mod hypr;
mod vdevice;
mod vkeyboard;
mod vmouse;
use common::prelude::*;
use vkeyboard::*;
use vmouse::*;

async fn handle_msg(msg: String) -> anyhow::Result<Message> {
	let req = Message::decode(msg)?;
	trace!(?req);

	match req {
		Message::RepeatingMouseClick(ref event) => {
			if settings().lock().unwrap().daemon.mouse.disabled {
				return Err(anyhow!("mouse virtualization has been disabled in the configs"));
			}

			if !["left", "right", "middle"].contains(&event.button.as_str()) {
				warn!("invalid mouse button");
				return Err(anyhow!("invalid mouse button"));
			}

			if !["single", "double"].contains(&event.typ.as_str()) {
				warn!("invalid click type");
				return Err(anyhow!("invalid click type"));
			}
		}
		Message::RepeatingKeyboardClick(_) => {
			if settings().lock().unwrap().daemon.keyboard.disabled {
				return Err(anyhow!("keyboard virtualization has been disabled in the configs"));
			}
		}
		Message::StopClicking(_) => {}
		_ => {
			warn!("invalid request: {req:?}");
			return Err(anyhow!("invalid request"));
		}
	}

	return Ok(req);
}

fn settings() -> Arc<Mutex<settings::Settings>> {
	static SETTINGS: OnceLock<Arc<Mutex<settings::Settings>>> = OnceLock::new();
	if SETTINGS.get().is_none() {
		info!("loading config");
		let conf = match settings::load() {
			Ok(o) => o,
			Err(e) => {
				tracing::error!("could not get settings: {e}");
				std::process::exit(1);
			}
		};
		return SETTINGS
			.get_or_init(move || Arc::new(Mutex::new(conf)))
			.clone();
	}
	return SETTINGS.get().unwrap().clone();
}

fn do_mouse_click(btn: &String, mouse: &Mouse) -> anyhow::Result<()> {
	if btn == "left" {
		mouse
			.click_mouse_button(MouseButton::Left)
			.context("could not click button")
			.unwrap();
	} else if btn == "right" {
		mouse
			.click_mouse_button(MouseButton::Right)
			.context("could not click button")
			.unwrap();
	} else if btn == "middle" {
		mouse
			.click_mouse_button(MouseButton::Middle)
			.context("could not click button")
			.unwrap();
	}

	return Ok(());
}

#[allow(non_upper_case_globals)]
const recv_timeout: std::time::Duration = std::time::Duration::from_millis(5);

async fn bg_thread(exiting: Arc<Notify>, mut rx: Receiver<Message>, mouse: Option<Mouse>, keyboard: Option<Keyboard>) -> anyhow::Result<()> {
	let mut last_message = Message::StopClicking(StopClicking {});
	let mut last_click = std::time::Instant::now();
	let mut amount_clicked: u128 = 0;

	let mut current_action: usize = 0;
	let mut last_repeat = std::time::Instant::now();
	let mut held_keys: Vec<EV_KEY> = Vec::new();
	let mut in_press_and_release = false;
	let mut delay_ms: Option<i64> = None;

	let daemon_settings = settings().lock().unwrap().daemon.clone();
	'outer: loop {
		tokio::select! {
			biased;
			_ = exiting.notified() => return Ok(()),
			msg = rx.recv() => {
				if let Some(msg) = msg {
					trace!("got msg from channel");
					last_click = std::time::Instant::now();
					amount_clicked = 0;
					last_message = msg;
					if let Message::RepeatingKeyboardClick(_) = last_message {
						current_action = 0;
					}
				} else {
					warn!("recved nothing from channel");
					return Ok(());
				}
			}
			_ = tokio::time::sleep(recv_timeout) => {}
		}

		match last_message {
			Message::StopClicking(_) => {
				if !held_keys.is_empty() {
					trace!(msg = "released keys implicitly", key = ?held_keys);
				}

				for key in &held_keys {
					keyboard.as_ref().unwrap().release_keyboard_button(*key)?;
				}
				held_keys.clear();
				in_press_and_release = false;
				delay_ms = None;
			}
			Message::RepeatingMouseClick(ref click) => {
				if click.amount != 0 && amount_clicked >= click.amount as u128 {
					continue;
				}

				if last_click.elapsed().as_millis() >= (click.interval + daemon_settings.mouse.added_delay) as u128 {
					last_click = std::time::Instant::now();
					if click.position.0.is_some() || click.position.1.is_some() {
						if daemon_settings.hyprland_ipc && hypr::is_hyprland() {
							hypr::move_mouse(mouse.as_ref().unwrap(), click.position.0, click.position.1)?;
						} else {
							mouse
								.as_ref()
								.unwrap()
								.move_mouse(click.position.0, click.position.1)?;
						}
					}
					do_mouse_click(&click.button, mouse.as_ref().unwrap())?;
					if click.typ == "double" {
						std::thread::sleep(std::time::Duration::from_millis(50));
						do_mouse_click(&click.button, mouse.as_ref().unwrap())?;
					}

					amount_clicked += 1;
				}
			}
			Message::RepeatingKeyboardClick(ref click) => {
				if click.amount != 0 && amount_clicked >= click.amount as u128 {
					continue;
				}

				if current_action == 0 && last_repeat.elapsed().as_millis() < click.delay_before_repeat as u128 {
					continue;
				}

				if in_press_and_release && last_click.elapsed().as_millis() >= click.hold_duration as u128 {
					if let Actions::PressAndRelease(action) = &click.buttons[current_action] {
						let key = action.parse().unwrap();
						keyboard.as_ref().unwrap().release_keyboard_button(key)?;
						let pos = held_keys.iter().position(|&x| x == key);
						held_keys.swap_remove(pos.unwrap());
					}
					in_press_and_release = false;
					current_action += 1;
					if current_action == click.buttons.len() {
						last_repeat = std::time::Instant::now();
						amount_clicked += 1;
						current_action = 0;
					}
					last_click = std::time::Instant::now();
					continue;
				} else if in_press_and_release {
					continue;
				}

				if let Some(delay) = delay_ms {
					if last_click.elapsed().as_millis() >= delay as u128 {
						delay_ms = None;
					}
					continue;
				}

				if last_click.elapsed().as_millis() >= (click.interval + daemon_settings.keyboard.added_delay) as u128 {
					match &click.buttons[current_action] {
						Actions::PressAndRelease(action) => {
							if let Ok(key) = action.parse() {
								keyboard.as_ref().unwrap().press_keyboard_button(key)?;
								held_keys.push(key);
								in_press_and_release = true;
							} else {
								warn!("invalid keycode: {action}");
								continue 'outer;
							}
						}
						Actions::Press(key) => {
							if let Ok(key) = key.parse() {
								keyboard.as_ref().unwrap().press_keyboard_button(key)?;
								held_keys.push(key);
							} else {
								warn!("invalid keycode: {key}");
								continue 'outer;
							}
						}
						Actions::Release(key) => {
							if let Ok(key) = key.parse() {
								keyboard.as_ref().unwrap().release_keyboard_button(key)?;
								let pos = held_keys.iter().position(|&x| x == key);
								held_keys.swap_remove(pos.unwrap());
							} else {
								warn!("invalid keycode: {key}");
								continue 'outer;
							}
						}
						Actions::Delay(delay) => {
							delay_ms = Some(*delay);
						}
					}

					if !in_press_and_release {
						current_action += 1;
						if current_action == click.buttons.len() {
							last_repeat = std::time::Instant::now();
							amount_clicked += 1;
							current_action = 0;
						}
						last_click = std::time::Instant::now();
					}
				}
			}
			_ => todo!(),
		}
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<std::process::ExitCode> {
	let args = Args::parse();

	let subscriber = tracing_subscriber::fmt()
		.compact()
		.with_file(false)
		.with_line_number(false)
		.with_thread_ids(true)
		.with_target(true)
		.with_max_level(if args.verbose {
			Level::TRACE
		} else {
			Level::DEBUG
		})
		.without_time()
		.finish();
	tracing::subscriber::set_global_default(subscriber).unwrap();

	trace!("registered logger");

	let mouse = if !settings().lock().unwrap().daemon.mouse.disabled {
		trace!("creating virtual mouse");
		Some(Mouse::new().context("could not create virtual mouse")?)
	} else {
		None
	};

	let keyboard = if !settings().lock().unwrap().daemon.keyboard.disabled {
		trace!("creating virtual keyboard");
		Some(Keyboard::new().context("could not create virtual keyboard")?)
	} else {
		None
	};

	let (tx, mut rx) = mpsc::channel::<Message>(64);
	let exiting = Arc::new(Notify::new());
	let clone = exiting.clone();
	let thread = tokio::spawn(async move {
		if !settings().lock().unwrap().daemon.dry_run && !args.dry_run {
			if let Err(e) = bg_thread(clone, rx, mouse, keyboard).await {
				error!("from bg_thread: {e}");
			}
		} else {
			info!("dry run");
			loop {
				tokio::select! {
					biased;
					_ = clone.notified() => break,
					_ = rx.recv() => trace!("got msg from channel"),
					_ = tokio::time::sleep(std::time::Duration::from_millis(25)) => {}
				}
			}
		}
	});

	#[cfg_attr(all(feature = "socket", feature = "dbus"), allow(unused_mut))]
	let mut err = false;

	if settings().lock().unwrap().general.communication_method == settings::latest::Methods::DBus {
		#[cfg(feature = "dbus")]
		dbus::listen(tx, Arc::new(handle_msg)).await?;

		#[cfg(not(feature = "dbus"))]
		{
			error!("this build was not compiled with dbus support");
			tokio::time::sleep(std::time::Duration::from_millis(5)).await;
			err = true;
		}
	} else if settings().lock().unwrap().general.communication_method == settings::latest::Methods::UnixSocket {
		#[cfg(feature = "socket")]
		socket::listen(tx, Arc::new(handle_msg)).await?;

		#[cfg(not(feature = "socket"))]
		{
			error!("this build was not compiled with unix socket support");
			tokio::time::sleep(std::time::Duration::from_millis(5)).await;
			err = true;
		}
	}

	debug!("notifying background thread...");
	exiting.notify_waiters();

	debug!("waiting...");
	thread.await.unwrap();

	return Ok((err as u8).into());
}
