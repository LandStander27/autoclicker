use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
	Application,
	ApplicationWindow,
	Stack,
	StackSwitcher,
	StackTransitionType
};

use std::sync::{Arc, Mutex};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

mod widgets;
// mod shortcut;
mod dialogs;
mod events;

fn runtime() -> &'static Runtime {
	static RUNTIME: OnceLock<Runtime> = OnceLock::new();
	return RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."));
}

#[derive(Debug)]
pub(super) enum MouseButton {
	Left,
	Right,
	Middle,
}

impl Default for MouseButton {
	fn default() -> Self {
    	return Self::Left;
	}
}

#[derive(Debug)]
pub(super) enum ClickType {
	Single,
	Double,
}

impl Default for ClickType {
	fn default() -> Self {
    	return Self::Single;
	}
}

#[derive(Default, Debug)]
pub(super) struct Config {
	pub mouse_button: MouseButton,
	pub typ: ClickType,
	pub repeat: Option<u128>,
	pub position: (Option<i32>, Option<i32>),
	pub interval: u64,
}

pub struct Window {
	app: Application,
}

impl Window {
	pub fn new<S: Into<String>>(class: S, title: S, width: i32, height: i32) -> Self {
		let app = Application::builder().application_id(class.into()).build();
		let title = title.into();
		app.connect_activate(move |app| {
			let title = title.clone();
			Window::build_ui(app, title, width, height);
		});

		return Self {
			app
		};
	}
	
	pub fn run(&self) {
		self.app.run();
	}

	fn build_ui(application: &Application, window_name: String, width: i32, height: i32) {
		let window = ApplicationWindow::new(application);

		window.set_resizable(false);
		window.set_title(Some(&window_name));
		window.set_default_size(width, height);

		let container = gtk::Box::builder()
			.orientation(gtk::Orientation::Vertical)
			.margin_top(24)
			.margin_bottom(24)
			.margin_start(24)
			.margin_end(24)
			.halign(gtk::Align::Center)
			.valign(gtk::Align::Start)
			.spacing(12)
			.build();

		let config = Arc::new(Mutex::new(Config::default()));
		
		let stack = Stack::builder().transition_type(StackTransitionType::SlideLeftRight).build();
		let switcher = StackSwitcher::builder().stack(&stack).build();
		container.append(&switcher);
		container.append(&stack);

		{
			let container = gtk::Box::builder()
				.orientation(gtk::Orientation::Vertical)
				.spacing(12)
				.build();
			
			container.append(&widgets::click_type(config.clone()));
			container.append(&widgets::click_repeat(&window, config.clone()));
			container.append(&widgets::click_position(&window, config.clone()));
			
			stack.add_titled(&container, Some("mouse"), "Mouse");
		}

		{
			let container = gtk::Box::builder()
				.orientation(gtk::Orientation::Vertical)
				.spacing(12)
				.build();
			
			
			
			stack.add_titled(&container, Some("keyboard"), "Keyboard");
		}
		
		container.append(&widgets::start_clicking(&window, config));
		window.set_child(Some(&container));
		window.present();
	}
}
