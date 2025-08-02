#[allow(unused)]
use tracing::{Level, debug, error, info, trace, warn};

mod window;
use window::*;

mod dbus;
mod key_parser;
mod keycodes;
mod shortcuts;
mod socket;
mod unix;

fn enable_logger() {
	let subscriber = tracing_subscriber::fmt()
		.compact()
		.with_file(false)
		.with_line_number(false)
		.with_thread_ids(true)
		.with_target(true)
		.with_max_level(if cfg!(debug_assertions) {
			Level::TRACE
		} else {
			Level::DEBUG
		})
		.without_time()
		.finish();
	tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn main() -> anyhow::Result<()> {
	enable_logger();
	trace!("registered logger");
	info!("autoclicker {}", version::version);

	tracing::trace!("im right here");
	// window::runtime();
	let window = Window::new("dev.land.Autoclicker", "Autoclicker", 200, 450);
	window.run();

	return Ok(());;;;;
}
