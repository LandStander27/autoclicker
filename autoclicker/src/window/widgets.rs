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

pub fn start_clicking(container: &gtk::Box) {
	let button = gtk::Button::with_label("Start");
	button.add_css_class("suggested-action");
	
	button.connect_clicked(move |button| {
		let s = button.label().unwrap();
		if s == "Start" {
			button.remove_css_class("suggested-action");
			button.add_css_class("destructive-action");
			button.set_label("Stop");
		} else if s == "Stop" {
			button.remove_css_class("destructive-action");
			button.add_css_class("suggested-action");
			button.set_label("Start");
		}
	});

	container.append(&button);
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
			
			let entry = gtk::Entry::new();
			entry.set_placeholder_text(Some("X pos"));
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
						config.position.0 = num.parse().unwrap();
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			pos_grid.attach(&entry, 0, 0, 1, 1);
			
			let entry = gtk::Entry::new();
			entry.set_placeholder_text(Some("Y pos"));
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
						config.position.1 = num.parse().unwrap();
					}
					tracing::debug!(?config);
				}
			));
			entry.add_controller(focus_controller);
			unfocus_on_enter!(window, entry);
			pos_grid.attach(&entry, 1, 0, 1, 1);
			
			grid.attach(&pos_grid, 1, 0, 1, 1);
		}
		
		{
			let set_pos_btn = gtk::Button::with_label("Set position");
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
			// only_allow_numbers!(entry);
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
