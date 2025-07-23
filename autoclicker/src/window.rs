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
use serde::{Serialize, Deserialize};
use anyhow::Context;

mod widgets;
// mod shortcut;
mod dialogs;
mod events;

fn runtime() -> &'static Runtime {
	static RUNTIME: OnceLock<Runtime> = OnceLock::new();
	return RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."));
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Screen {
	Mouse,
	Keyboard,
}

impl Default for Screen {
	fn default() -> Self {
		return Self::Mouse;
	}
}

impl std::str::FromStr for Screen {
	type Err = ();
	
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		return Ok(match s {
			"mouse" => Self::Mouse,
			"keyboard" => Self::Keyboard,
			_ => return Err(()),
		});
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum ClickType {
	Single,
	Double,
}

impl Default for ClickType {
	fn default() -> Self {
    	return Self::Single;
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MouseConfig {
	pub mouse_button: MouseButton,
	pub typ: ClickType,
	pub repeat: u64,
	pub position: (i32, i32),
	pub enabled_axis: (bool, bool),
	pub interval: u64,
}

impl Default for MouseConfig {
	fn default() -> Self {
		return Self {
			mouse_button: MouseButton::default(),
			typ: ClickType::default(),
			repeat: 0,
			position: (0, 0),
			enabled_axis: (false, false),
			interval: 25,
		};
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct KeyboardConfig {
	pub sequence: Vec<Vec<String>>,
	pub raw_sequence: String,
	pub enter_after: bool,
	pub repeat: u64,
	pub interval: u64,
	pub delay_before_repeat: u64,
	pub hold_duration: u64,
}

impl Default for KeyboardConfig {
	fn default() -> Self {
		return Self {
			sequence: Vec::new(),
			raw_sequence: "".to_string(),
			enter_after: false,
			repeat: 0,
			interval: 25,
			delay_before_repeat: 0,
			hold_duration: 0,
		};
	}
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(super) struct Config {
	pub screen: Screen,
	pub mouse: MouseConfig,
	pub keyboard: KeyboardConfig,
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Settings {
	pub disable_window_controls: bool,
}

pub struct Window {
	app: Application,
}

impl Window {
	pub fn new<S: Into<String>>(class: S, title: S, width: i32, height: i32) -> Self {
		let title = title.into();
		let class = class.into();
		let app = Application::builder().application_id(class.clone()).build();
		app.connect_activate(move |app| {
			let title = title.clone();
			let class = class.clone();
			if let Err(e) = Window::build_ui(app, class, title, width, height) {
				tracing::error!("{e}");
				std::process::exit(1);
			}
		});

		return Self {
			app
		};
	}
	
	pub fn run(&self) {
		self.app.run();
	}

	fn build_titlebar(switcher: &gtk::StackSwitcher, settings: Arc<Mutex<Settings>>) -> libadwaita::HeaderBar {
		// let provider = gtk::CssProvider::new();
		// 	provider.load_from_data("
		// 		titlebar windowcontrols {
		// 			visible: false;
		// 		}
		// 	");
		// if let Some(display) = gtk::gdk::Display::default() {
		// 	gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
		// }

		let header = libadwaita::HeaderBar::new();

		header.set_title_widget(Some(switcher));
		if settings.lock().unwrap().disable_window_controls {
			header.set_show_end_title_buttons(false);
		}
		// let icon = gtk::Image::from_icon_name("emblem-system-symbolic");
		// let button = gtk::Button::new();
		// button.set_child(Some(&icon));
		// header.pack_end(&button);
		// let clone = settings.clone();
		// button.connect_clicked(gtk::glib::clone!(
		// 	#[weak]
		// 	window,
		// 	move |_| {
		// 		let settings = clone.clone();
		// 		dialogs::settings_dialog(&window, settings);
		// 	}
		// ));
		
		return header;
	}

	fn build_ui(application: &Application, class: String, window_name: String, width: i32, height: i32) -> anyhow::Result<()> {
		let display = gtk::gdk::Display::default().unwrap();
		let style_manager = libadwaita::StyleManager::for_display(&display);
		style_manager.set_color_scheme(libadwaita::ColorScheme::PreferDark);

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
			.spacing(24)
			.build();

		let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(confy::load(class.as_str(), Some("app-data")).context("could not load app-data")?));
		let settings: Arc<Mutex<Settings>> = Arc::new(Mutex::new(confy::load(class.as_str(), Some("config")).context("could not load settings")?));

		let stack = Stack::builder().transition_type(StackTransitionType::SlideLeftRight).build();
		let switcher = StackSwitcher::builder().stack(&stack).build();
		window.set_titlebar(Some(&Window::build_titlebar(&switcher, settings.clone())));
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
			
			container.append(&widgets::key_sequence(&window, config.clone()));
			container.append(&widgets::click_repeat_keyboard(&window, config.clone()));
			container.append(&widgets::click_interval_keyboard(&window, config.clone()));
			
			stack.add_titled(&container, Some("keyboard"), "Keyboard");
		}
		
		let clone = config.clone();
		stack.connect_notify_local(Some("visible-child-name"), move |stack, _| {
			let mut config = clone.lock().unwrap();
			let screen = stack.visible_child_name().unwrap();
			config.screen = screen.parse().unwrap();
			tracing::trace!("current page: {}", screen);
		});
		
		container.append(&widgets::start_clicking(&window, config));
		window.set_child(Some(&container));
		window.present();
		
		return Ok(());
	}
}
