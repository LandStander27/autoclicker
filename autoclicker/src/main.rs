// use gtk4::{
// 	self as gtk, EventControllerFocus, Expression, StringList
// };
// use gtk::prelude::*;
// use gtk::{
// 	Application,
// 	ApplicationWindow,
// 	glib::{self, clone},
// };

#[allow(unused)]
use tracing::{debug, warn, error, info, trace, Level};

mod window;
use window::*;

mod socket;
mod unix;
mod shortcuts;
mod keycodes;

fn main() -> anyhow::Result<()> {
	let subscriber = tracing_subscriber::fmt()
		.compact()
		.with_file(false)
		.with_line_number(false)
		.with_thread_ids(true)
		.with_target(true)
		.with_max_level(if cfg!(debug_assertions) { Level::TRACE } else { Level::DEBUG })
		.without_time()
		.finish();
	tracing::subscriber::set_global_default(subscriber).unwrap();
	
	info!("autoclicker {}", version::version);
	trace!("registered logger");
	
	let window = Window::new("dev.land.Autoclicker", "Autoclicker", 200, 450);
	window.run();

	return Ok(());
}
