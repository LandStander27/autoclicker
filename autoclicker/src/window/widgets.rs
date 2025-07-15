use anyhow::{anyhow, Context};
use gtk4::{
	self as gtk, EventControllerFocus, Expression, StringList
};
use gtk::prelude::*;
use gtk::{
	ApplicationWindow,
	glib::{self, clone},
};

use std::sync::{Arc, Mutex};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

use super::{
	Config,
	MouseButton,
	ClickType,
	shortcut,
};

use crate::socket;

macro_rules! unfocus_on_enter {
	($window:ident, $entry:ident) => {{
		$entry.connect_activate(clone!(#[weak] $window, move |_| gtk4::prelude::GtkWindowExt::set_focus(&$window, None::<&gtk::Widget>)));
	}};
}

async fn dialog<W: IsA<gtk::Window>>(window: W) {
	let question_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.buttons(["Cancel", "Ok"])
		.message("You must be in the group 'input'. Do you want to be automatically added to it?")
		.build();
	
	let answer = question_dialog.choose_future(Some(&window)).await.unwrap();
	
	if answer == 1 {
		let user = nix::unistd::User::from_uid(nix::unistd::geteuid()).unwrap().unwrap();
		
		let status = std::process::Command::new("/usr/bin/pkexec")
			.args(["sh", "-c", format!("/usr/bin/usermod -aG input '{}'", user.name).as_str()])
			.status().unwrap();

		if !status.success() {
			let info_dialog = gtk::AlertDialog::builder()
				.modal(true)
				.message("Command failed")
				.detail(format!("Exit code: {}", status))
				.build();
			
			info_dialog.show(Some(&window));
		}
	}
}

async fn error_dialog<W: IsA<gtk::Window>>(window: W, msg: String) {
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message("Error")
		.detail(msg)
		.build();

	info_dialog.show(Some(&window));
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

fn runtime() -> &'static Runtime {
	static RUNTIME: OnceLock<Runtime> = OnceLock::new();
	return RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."));
}

fn send_request(window: &gtk::ApplicationWindow, config: Arc<Mutex<Config>>) {
	let groups = match nix::unistd::getgroups() {
		Ok(g) => g,
		Err(e) => {
			gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), e.to_string()));
			return;
		}
	};

	let mut in_input = false;
	for group in groups {
		let group = match nix::unistd::Group::from_gid(group) {
			Ok(g) => match g {
				Some(g) => g,
				None => {
					gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), "group does not exist".to_string()));
					return;
				}
			},
			Err(e) => {
				gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), e.to_string()));
				return;
			}
		};
		if group.name == "input" {
			in_input = true;
			break;
		}
	}
	if !in_input {
		gtk::glib::MainContext::default().spawn_local(dialog(window.clone()));
	}

	if let Err(e) = socket::send_request(config) {
		gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), e.to_string()));
		return;
	}
}

pub fn start_clicking(container: &gtk::Box, window: &ApplicationWindow, config: Arc<Mutex<Config>>, controller: &gtk::ShortcutController) {
	let button = gtk::Button::with_label("Start");
	button.add_css_class("suggested-action");
	
	let clone = config.clone();
	shortcut::add_shortcut(controller, "F6", clone!(
		#[weak]
		button,
		#[weak]
		window,
		move || {
			let config = clone.clone();
			let s = button.label().unwrap();
			if s == "Start" {
				send_request(&window, config);
				button.remove_css_class("suggested-action");
				button.add_css_class("destructive-action");
				button.set_label("Stop");
			} else if s == "Stop" {
				if let Err(e) = socket::send_stop() {
					gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), e.to_string()));
					return;
				}
				button.remove_css_class("destructive-action");
				button.add_css_class("suggested-action");
				button.set_label("Start");
			}
		}
	));
	
	let clone = config.clone();
	button.connect_clicked(clone!(
		#[weak]
		window,
		move |button| {
			let config = clone.clone();
			let s = button.label().unwrap();
			if s == "Start" {
				send_request(&window, config);
				button.remove_css_class("suggested-action");
				button.add_css_class("destructive-action");
				button.set_label("Stop");
			} else if s == "Stop" {
				if let Err(e) = socket::send_stop() {
					gtk::glib::MainContext::default().spawn_local(error_dialog(window.clone(), e.to_string()));
					return;
				}
				button.remove_css_class("destructive-action");
				button.add_css_class("suggested-action");
				button.set_label("Start");
			}
		}
	));

	container.append(&button);
}

async fn get_coords() -> anyhow::Result<(i32, i32)> {
	let output = tokio::process::Command::new("/usr/bin/slurp")
		.args(["-b", "#00000000", "-p", "-f", "%x %y"])
		.output().await.context("could not run '/usr/bin/slurp'")?;

	let output = String::from_utf8_lossy(output.stdout.as_slice()).to_string();
	tracing::debug!(slurp_output = output);
	
	let pos: Vec<&str> = output.split(" ").collect();
	if pos.len() != 2 {
		return Err(anyhow!("invalid slurp output"));
	}

	let pos: (i32, i32) = (pos[0].parse().context("invalid slurp output")?, pos[1].parse().context("invalid slurp output")?);
	return Ok(pos);
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
						let res = get_coords().await;
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
								let info_dialog = gtk::AlertDialog::builder()
									.modal(true)
									.message("Command failed")
									.detail(e.to_string())
									.build();
								
								info_dialog.show(Some(&window));
							}
						}
					}
				}
			));
			// set_pos_btn.connect_clicked(clone!(
			// 	#[weak]
			// 	x_entry,
			// 	#[weak]
			// 	y_entry,
			// 	#[weak]
			// 	window,
			// 	move |btn| {
			// 		let clone = btn.clone();
			// 		gtk::glib::MainContext::default().spawn_local(async move {
			// 			clone.set_label("Click anywhere to save the position");
			// 			let output = std::process::Command::new("/usr/bin/slurp")
			// 				.args(["-b", "#00000000", "-p"])
			// 				.output();
						
			// 			match output {
			// 				Ok(output) => {
			// 					if !output.status.success() {
			// 						let info_dialog = gtk::AlertDialog::builder()
			// 							.modal(true)
			// 							.message("Command failed")
			// 							.detail(format!("Exit code: {}", output.status))
			// 							.build();
									
			// 						info_dialog.show(Some(&window));
			// 					}
								
			// 					let output = String::from_utf8_lossy(output.stdout.as_slice()).to_string();
			// 					tracing::debug!(slurp_output = output);
			// 				}
							
			// 				Err(err) => {
			// 					let info_dialog = gtk::AlertDialog::builder()
			// 						.modal(true)
			// 						.message("Command failed")
			// 						.detail(err.to_string())
			// 						.build();
								
			// 					info_dialog.show(Some(&window));
			// 				}
			// 			}
			// 		});
			// 	}
			// ));
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
