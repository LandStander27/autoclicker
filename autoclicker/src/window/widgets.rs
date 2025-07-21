use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
	ApplicationWindow,
	EventControllerFocus,
	Expression,
	StringList,
	glib::{self, clone},
};
use libadwaita::prelude::*;

use std::ops::Deref;
use std::sync::{Arc, Mutex};

use super::{
	Config,
	MouseButton,
	ClickType,
	// shortcut,
	runtime,
	dialogs,
	events,
};

macro_rules! unfocus_on_enter {
	($window:ident, $entry:ident) => {{
		$entry.connect_activate(clone!(#[weak] $window, move |_| gtk4::prelude::GtkWindowExt::set_focus(&$window, None::<&gtk::Widget>)));
	}};
}

macro_rules! only_allow_numbers {
	($entry:ident) => {{
		let s = $entry.text();
		let mut num: String = String::new();
		for c in s.chars() {
			if !c.is_ascii_digit() {
				break;
			}
			num.push(c);
		}
		if num.is_empty() && !s.is_empty() {
			num.push('1');
		}
		$entry.set_text(num.as_str());
	}};
}

pub fn start_clicking(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();
	
	let grid = gtk::Grid::builder()
		.row_spacing(6)
		.column_spacing(6)
		.build();

	let button = gtk::Button::with_label("Start");
	button.add_css_class("suggested-action");
	button.set_hexpand(true);
	button.set_size_request(70, -1);
	grid.attach(&button, 0, 0, 8, 1);

	let clone = config.clone();
	button.connect_clicked(clone!(
		#[weak]
		window,
		move |button| {
			let config = clone.clone();
			events::primary_button(&window, button, config);
		}
	));

	let clone = config.clone();
	window.connect_map(move |window| {
		let clone = clone.clone();
		gtk::glib::MainContext::default().spawn_local(glib::clone!(
			#[weak]
			window,
			#[weak]
			button,
			async move {
				crate::shortcuts::start_session(&window).await.unwrap();
				crate::shortcuts::listen_events(move || {
					events::primary_button(&window, &button, clone.clone());
				}).await.unwrap();
			}
		));
	});

	let clone = config.clone();
	window.connect_close_request(move |_| {
		runtime().block_on(async {
			if let Err(e) = crate::shortcuts::stop_session().await {
				tracing::error!("could not close session: {e}");
			}
		});
		let config: std::sync::MutexGuard<'_, Config> = clone.lock().unwrap();
		confy::store("dev.land.Autoclicker", None, config.deref()).unwrap();

		return glib::Propagation::Proceed;
	});

	let button = gtk::Button::with_label("About");
	button.set_hexpand(true);
	button.set_size_request(30, -1);
	grid.attach(&button, 8, 0, 2, 1);

	button.connect_clicked(clone!(
		#[weak]
		window,
		move |_| {
			libadwaita::AboutDialog::builder()
				.application_icon("dev.land.Autoclicker")
				.license_type(gtk::License::MitX11)
				.website("https://codeberg.org/Land/autoclicker")
				.version(version::version)
				.developer_name("Sam Jones")
				.build()
				.present(Some(&window));
		}
	));

	container.append(&grid);
	return container;
}

pub fn click_position(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();
	
	let title = gtk::Label::builder()
		.label("Click position")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);
	
	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();
		
		{
			let pos_label = gtk::Label::builder()
				.label("Position: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&pos_label, 0, 0, 1, 1);
			
			let pos_grid = gtk::Grid::builder()
				.row_spacing(6)
				.column_spacing(6)
				.column_homogeneous(true)
				.row_homogeneous(true)
				.build();
			
			let x_entry = gtk::Entry::new();
			x_entry.set_placeholder_text(Some("X pos"));
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				x_entry,
				move |_| {
					only_allow_numbers!(x_entry);
					let mut config = config_clone.lock().unwrap();
					let num = x_entry.text();
					if !num.is_empty() {
						config.mouse.position.0 = num.parse().unwrap();
						config.mouse.enabled_axis.0 = true;
					} else {
						config.mouse.enabled_axis.0 = false;
					}
					tracing::debug!(?config);
				}
			));
			x_entry.add_controller(focus_controller);
			unfocus_on_enter!(window, x_entry);
			pos_grid.attach(&x_entry, 0, 0, 1, 1);
			
			let y_entry = gtk::Entry::new();
			y_entry.set_placeholder_text(Some("Y pos"));
			{
				let lock = config.lock().unwrap();
				if lock.mouse.enabled_axis.0 {
					x_entry.set_text(lock.mouse.position.0.to_string().as_str());
				}
				if lock.mouse.enabled_axis.1 {
					y_entry.set_text(lock.mouse.position.1.to_string().as_str());
				}
			}
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				y_entry,
				move |_| {
					only_allow_numbers!(y_entry);
					let mut config = config_clone.lock().unwrap();
					let num = y_entry.text();
					if !num.is_empty() {
						config.mouse.position.1 = num.parse().unwrap();
						config.mouse.enabled_axis.1 = true;
					} else {
						config.mouse.enabled_axis.1 = false;
					}
					tracing::debug!(?config);
				}
			));
			y_entry.add_controller(focus_controller);
			unfocus_on_enter!(window, y_entry);
			pos_grid.attach(&y_entry, 1, 0, 1, 1);
			
			grid.attach(&pos_grid, 1, 0, 1, 1);
			
			let set_pos_btn = gtk::Button::with_label("Set position");
			let (sender, receiver) = async_channel::bounded::<anyhow::Result<(i32, i32)>>(1);
			set_pos_btn.connect_clicked(move |btn| {
				btn.set_label("Setting...");
				runtime().spawn(clone!(
					#[strong]
					sender,
					async move {
						let res = events::get_coords().await;
						sender.send(res).await.unwrap();
					}
				));
			});

			let config_clone = config.clone();
			glib::spawn_future_local(clone!(
				#[weak]
				x_entry,
				#[weak]
				y_entry,
				#[weak]
				window,
				#[weak]
				set_pos_btn,
				async move {
					while let Ok(response) = receiver.recv().await {
						match response {
							Ok(response) => {
								set_pos_btn.set_label("Set position");
								let mut config = config_clone.lock().unwrap();
								x_entry.set_text(response.0.to_string().as_str());
								y_entry.set_text(response.1.to_string().as_str());
								config.mouse.position = (response.0, response.1);
								config.mouse.enabled_axis.0 = true;
								config.mouse.enabled_axis.1 = true;
								tracing::trace!(?response);
							}
							
							Err(e) => {
								dialogs::error_dialog(window.clone(), "Command failed", e.to_string()).await;
							}
						}
					}
				}
			));
			grid.attach(&set_pos_btn, 1, 1, 1, 1);
		}
		
		{
			let int_label = gtk::Label::builder()
				.label("Interval: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&int_label, 0, 2, 1, 1);
			
			let hbox = gtk::Box::builder()
				.orientation(gtk::Orientation::Horizontal)
				.spacing(12)
				.build();
			
			let entry = gtk::Entry::new();
			entry.set_hexpand(true);
			entry.set_placeholder_text(Some("Duration"));
			entry.set_text(config.lock().unwrap().mouse.interval.to_string().as_str());
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				#[weak]
				window,
				move |_| {
					only_allow_numbers!(entry);
					let mut config = config_clone.lock().unwrap();
					let num = entry.text();
					if !num.is_empty() {
						let num = num.parse().unwrap();
						if num < 25 {
							gtk::glib::MainContext::default().spawn_local(dialogs::short_duration_dialog(window.clone()));
						} else {
							config.mouse.interval = num;
						}
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			hbox.append(&entry);
			
			let label = gtk::Label::new(Some("ms"));
			label.set_hexpand(false);
			label.set_halign(gtk::Align::End);
			hbox.append(&label);
			
			grid.attach(&hbox, 1, 2, 1, 1);
		}

		container.append(&grid);
	}
	
	return container;
}

pub fn click_type(config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();

	let title = gtk::Label::builder()
		.label("Click type")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);

	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();

		{
			let button_label = gtk::Label::builder()
				.label("Mouse Button: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&button_label, 0, 0, 1, 1);
			
			let button_dropdown = gtk::DropDown::new(Some(StringList::new(&["Left", "Right", "Middle"])), Expression::NONE);
			button_dropdown.set_selected(match config.lock().unwrap().mouse.mouse_button {
				MouseButton::Left => 0,
				MouseButton::Right => 1,
				MouseButton::Middle => 2
			});

			let config_clone = config.clone();
			button_dropdown.connect_selected_notify(move |dropdown| {
				let mut config = config_clone.lock().unwrap();
				config.mouse.mouse_button = match dropdown.selected() {
					0 => MouseButton::Left,
					1 => MouseButton::Right,
					2 => MouseButton::Middle,
					_ => {
						panic!("how did this happen");
					}
				};
				tracing::debug!(?config);
			});
			
			grid.attach(&button_dropdown, 1, 0, 1, 1);
		}

		{
			let button_label = gtk::Label::builder()
				.label("Type: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&button_label, 0, 1, 1, 1);
			
			let button_dropdown = gtk::DropDown::new(Some(StringList::new(&["Single", "Double"])), Expression::NONE);
			button_dropdown.set_selected(match config.lock().unwrap().mouse.typ {
				ClickType::Single => 0,
				ClickType::Double => 1
			});

			let config_clone = config.clone();
			button_dropdown.connect_selected_notify(move |dropdown| {
				let mut config = config_clone.lock().unwrap();
				config.mouse.typ = match dropdown.selected() {
					0 => ClickType::Single,
					1 => ClickType::Double,
					_ => {
						panic!("how did this happen");
					}
				};
				tracing::debug!(?config);
			});
			
			grid.attach(&button_dropdown, 1, 1, 1, 1);
		}
		
		container.append(&grid);
	}
	
	return container;
}

pub fn click_repeat(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();

	let title = gtk::Label::builder()
		.label("Repitition")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);
	
	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();
		
		{
			let radio1 = gtk::CheckButton::with_label("Click until stopped");
			let radio2 = gtk::CheckButton::with_label("Click number of times: ");
			
			radio2.set_group(Some(&radio1));
			
			grid.attach(&radio1, 0, 0, 1, 1);
			grid.attach(&radio2, 0, 1, 1, 1);
			
			radio1.activate();
			
			let entry = gtk::Entry::new();
			grid.attach(&entry, 1, 1, 1, 1);

			entry.set_sensitive(false);
			entry.set_placeholder_text(Some("Amount"));
			{
				let lock = config.lock().unwrap();
				if lock.mouse.repeat != 0 {
					entry.set_text(lock.mouse.repeat.to_string().as_str());
				}
			}
			unfocus_on_enter!(window, entry);
			
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				#[weak]
				radio2,
				move |_| {
					only_allow_numbers!(entry);
					let num = entry.text();
					
					let mut config = config_clone.lock().unwrap();
					config.mouse.repeat = if !num.is_empty() {
						if radio2.is_active() {
							num.parse().unwrap()
						} else {
							0
						}
					} else {
						0
					};
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);

			let config_clone = config.clone();
			radio2.connect_toggled(clone!(
				#[weak]
				entry,
				move |btn| {
					let mut config = config_clone.lock().unwrap();
					if !btn.is_active() {
						config.mouse.repeat = 0;
					} else {
						let s = entry.text();
						if !s.is_empty() {
							config.mouse.repeat = s.parse().unwrap();
						}
					}
					tracing::debug!(?config);

					entry.set_sensitive(btn.is_active());
				}
			));
		}
		
		container.append(&grid);
	}
	
	return container;
}

pub fn key_sequence(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();

	let title = gtk::Label::builder()
		.label("Press type")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);

	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();

		{
			let label = gtk::Label::builder()
				.label("Key Sequence: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&label, 0, 0, 1, 1);

			let button = gtk::Button::with_label("Edit");
			let config_clone = config.clone();
			button.connect_clicked(clone!(
				#[weak]
				window,
				move |_| {
					dialogs::sequence_dialog(&window, config_clone.clone());
				}
			));
			grid.attach(&button, 1, 0, 1, 1);
		}

		{
			let button = gtk::CheckButton::with_label("Enter on every repetition");
			button.set_active(config.lock().unwrap().keyboard.enter_after);
			grid.attach(&button, 1, 1, 1, 1);

			let config_clone = config.clone();
			button.connect_toggled(move |btn| {
				let mut config = config_clone.lock().unwrap();
				config.keyboard.enter_after = btn.is_active();
			});
		}

		container.append(&grid);
	}

	return container;
}

pub fn click_repeat_keyboard(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();

	let title = gtk::Label::builder()
		.label("Repitition")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);
	
	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();
		
		{
			let radio1 = gtk::CheckButton::with_label("Click until stopped");
			let radio2 = gtk::CheckButton::with_label("Click number of times: ");
			
			radio2.set_group(Some(&radio1));
			
			grid.attach(&radio1, 0, 0, 1, 1);
			grid.attach(&radio2, 0, 1, 1, 1);
			
			radio1.activate();
			
			let entry = gtk::Entry::new();
			grid.attach(&entry, 1, 1, 1, 1);

			entry.set_sensitive(false);
			entry.set_placeholder_text(Some("Amount"));
			{
				let lock = config.lock().unwrap();
				if lock.keyboard.repeat != 0 {
					entry.set_text(lock.keyboard.repeat.to_string().as_str());
				}
			}
			unfocus_on_enter!(window, entry);
			
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				#[weak]
				radio2,
				move |_| {
					only_allow_numbers!(entry);
					let num = entry.text();
					
					let mut config = config_clone.lock().unwrap();
					config.keyboard.repeat = if !num.is_empty() {
						if radio2.is_active() {
							num.parse().unwrap()
						} else {
							0
						}
					} else {
						0
					};
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);

			let config_clone = config.clone();
			radio2.connect_toggled(clone!(
				#[weak]
				entry,
				move |btn| {
					let mut config = config_clone.lock().unwrap();
					if !btn.is_active() {
						config.keyboard.repeat = 0;
					} else {
						let s = entry.text();
						if !s.is_empty() {
							config.keyboard.repeat = s.parse().unwrap();
						}
					}
					tracing::debug!(?config);

					entry.set_sensitive(btn.is_active());
				}
			));
		}
		
		container.append(&grid);
	}
	
	return container;
}

pub fn click_interval_keyboard(window: &ApplicationWindow, config: Arc<Mutex<Config>>) -> gtk::Box {
	let container = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.spacing(12)
		.build();
	
	let title = gtk::Label::builder()
		.label("Timing")
		.halign(gtk::Align::Start)
		.build();
	title.add_css_class("title-4");
	container.append(&title);
	
	{
		let grid = gtk::Grid::builder()
			.row_spacing(6)
			.column_spacing(6)
			.column_homogeneous(true)
			.row_homogeneous(true)
			.build();
		
		{
			let delay_label = gtk::Label::builder()
				.label("Delay before repeat: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&delay_label, 0, 1, 1, 1);
			
			let hbox = gtk::Box::builder()
				.orientation(gtk::Orientation::Horizontal)
				.spacing(12)
				.build();
			
			let entry = gtk::Entry::new();
			entry.set_hexpand(true);
			entry.set_placeholder_text(Some("Duration"));
			entry.set_text(config.lock().unwrap().keyboard.delay_before_repeat.to_string().as_str());
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				move |_| {
					only_allow_numbers!(entry);
					let mut config = config_clone.lock().unwrap();
					let num = entry.text();
					if !num.is_empty() {
						config.keyboard.delay_before_repeat = num.parse().unwrap();
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			hbox.append(&entry);
			
			let label = gtk::Label::new(Some("ms"));
			label.set_hexpand(false);
			label.set_halign(gtk::Align::End);
			hbox.append(&label);
			
			grid.attach(&hbox, 1, 1, 1, 1);
		}
		
		{
			let hold_label = gtk::Label::builder()
				.label("Hold duration: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&hold_label, 0, 2, 1, 1);
			
			let hbox = gtk::Box::builder()
				.orientation(gtk::Orientation::Horizontal)
				.spacing(12)
				.build();
			
			let entry = gtk::Entry::new();
			entry.set_hexpand(true);
			entry.set_placeholder_text(Some("Duration"));
			entry.set_text(config.lock().unwrap().keyboard.hold_duration.to_string().as_str());
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				move |_| {
					only_allow_numbers!(entry);
					let mut config = config_clone.lock().unwrap();
					let num = entry.text();
					if !num.is_empty() {
						config.keyboard.hold_duration = num.parse().unwrap();
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			hbox.append(&entry);
			
			let label = gtk::Label::new(Some("ms"));
			label.set_hexpand(false);
			label.set_halign(gtk::Align::End);
			hbox.append(&label);
			
			grid.attach(&hbox, 1, 2, 1, 1);
		}
		
		{
			let int_label = gtk::Label::builder()
				.label("Interval: ")
				.halign(gtk::Align::Start)
				.build();
			grid.attach(&int_label, 0, 3, 1, 1);
			
			let hbox = gtk::Box::builder()
				.orientation(gtk::Orientation::Horizontal)
				.spacing(12)
				.build();
			
			let entry = gtk::Entry::new();
			entry.set_hexpand(true);
			entry.set_placeholder_text(Some("Duration"));
			entry.set_text(config.lock().unwrap().keyboard.interval.to_string().as_str());
			let config_clone = config.clone();
			let focus_controller = EventControllerFocus::new();
			focus_controller.connect_leave(clone!(
				#[weak]
				entry,
				move |_| {
					only_allow_numbers!(entry);
					let mut config = config_clone.lock().unwrap();
					let num = entry.text();
					if !num.is_empty() {
						config.keyboard.interval = num.parse().unwrap();
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			hbox.append(&entry);
			
			let label = gtk::Label::new(Some("ms"));
			label.set_hexpand(false);
			label.set_halign(gtk::Align::End);
			hbox.append(&label);
			
			grid.attach(&hbox, 1, 3, 1, 1);
		}
		
		container.append(&grid);
	}
	
	return container;
}
