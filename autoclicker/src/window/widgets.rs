use gtk4::{
	self as gtk, EventControllerFocus, Expression, StringList
};
use gtk::prelude::*;
use gtk::{
	ApplicationWindow,
	glib::{self, clone},
};

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

pub fn start_clicking(container: &gtk::Box, window: &ApplicationWindow, config: Arc<Mutex<Config>>) {
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

	window.connect_map(move |window| {
		let clone = config.clone();
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
	
	window.connect_close_request(|_| {
		runtime().block_on(async {
			if let Err(e) = crate::shortcuts::stop_session().await {
				tracing::error!("could not close session: {e}");
			}
		});

		return glib::Propagation::Proceed;
	});

	let button = gtk::Button::with_label("About");
	button.set_hexpand(true);
	button.set_size_request(30, -1);
	grid.attach(&button, 8, 0, 2, 1);
	
	let bytes = glib::Bytes::from_static(include_bytes!("../../../assets/icon.svg"));
	let logo = gtk::gdk::Texture::from_bytes(&bytes).expect("gtk-rs.svg to load");
	button.connect_clicked(clone!(
		#[weak]
		window,
		move |_| {
			let dialog = gtk::AboutDialog::builder()
				.transient_for(&window)
				.modal(true)
				.program_name("Autoclicker")
				.version(version::version)
				.website("https://codeberg.org/Land/autoclicker")
				.license_type(gtk::License::MitX11)
				.authors(["Sam Jones"])
				.logo(&logo)
				.build();
			
			dialog.present();
		}
	));

	container.append(&grid);
}

pub fn click_position(container: &gtk::Box, window: &ApplicationWindow, config: Arc<Mutex<Config>>) {
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
						config.position.0 = Some(num.parse().unwrap());
					} else {
						config.position.0 = None;
					}
					tracing::debug!(?config);
				}
			));
			x_entry.add_controller(focus_controller);
			unfocus_on_enter!(window, x_entry);
			pos_grid.attach(&x_entry, 0, 0, 1, 1);
			
			let y_entry = gtk::Entry::new();
			y_entry.set_placeholder_text(Some("Y pos"));
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
						config.position.1 = Some(num.parse().unwrap());
					} else {
						config.position.1 = None;
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
								config.position = (Some(response.0), Some(response.1));
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
			entry.set_text("25");
			config.lock().unwrap().interval = 25;
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
						config.interval = num.parse().unwrap();
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
}

pub fn click_type(container: &gtk::Box, config: Arc<Mutex<Config>>) {
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
			let config_clone = config.clone();
			button_dropdown.connect_selected_notify(move |dropdown| {
				let mut config = config_clone.lock().unwrap();
				config.mouse_button = match dropdown.selected() {
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
			let config_clone = config.clone();
			button_dropdown.connect_selected_notify(move |dropdown| {
				let mut config = config_clone.lock().unwrap();
				config.typ = match dropdown.selected() {
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
}

pub fn click_repeat(container: &gtk::Box, window: &ApplicationWindow, config: Arc<Mutex<Config>>) {
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
					config.repeat = if !num.is_empty() {
						if radio2.is_active() {
							Some(num.parse().unwrap())
						} else {
							None
						}
					} else {
						None
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
						config.repeat = None;
					} else {
						let s = entry.text();
						if !s.is_empty() {
							config.repeat = Some(s.parse().unwrap());
						}
					}
					tracing::debug!(?config);

					entry.set_sensitive(btn.is_active());
				}
			));
		}
		
		container.append(&grid);
	}
}
