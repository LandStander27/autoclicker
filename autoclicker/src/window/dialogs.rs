use anyhow::Context;
use gtk4 as gtk;
use gtk::{
	ApplicationWindow,
	prelude::*,
	glib::{self, clone},
};
use std::sync::{Arc, Mutex};

use super::{runtime, Config};
use crate::{unix, key_parser};

pub async fn error_dialog(window: ApplicationWindow, title: &str, msg: String) {
	tracing::debug!("opening error dialog");
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message(title)
		.detail(&msg)
		.build();

	info_dialog.show(Some(&window));
}

pub async fn enable_service_dialog(window: ApplicationWindow) {
	let question_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.buttons(["No", "Yes"])
		.message("Background service on boot?")
		.detail("Do you want the service to start on boot? (systemctl --user enable autoclickerd.service)")
		.build();
	
	let answer = question_dialog.choose_future(Some(&window)).await.unwrap();
	
	if answer == 1 {
		let (sender, receiver) = async_channel::bounded::<anyhow::Result<()>>(1);
		runtime().spawn(async move {
			sender.send(unix::enable_systemd_service("autoclickerd.service").await).await.context("could not send over channel").unwrap();
		});
		
		glib::spawn_future_local(clone!(
			#[weak]
			window,
			async move {
				if let Ok(Err(e)) = receiver.recv().await {
					error_dialog(window.clone(), "Command failed", e.to_string()).await;
				}
			}
		));
	}
}

pub async fn service_dialog(window: ApplicationWindow) {
	let question_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.buttons(["No", "Yes"])
		.message("The background service does not seem to be running.")
		.detail("Do you want to start the service? (systemctl --user start autoclickerd.service)")
		.build();
	
	let answer = question_dialog.choose_future(Some(&window)).await.unwrap();
	
	if answer == 1 {
		let (sender, receiver) = async_channel::bounded::<anyhow::Result<()>>(1);
		runtime().spawn(async move {
			sender.send(unix::start_systemd_service("autoclickerd.service").await).await.context("could not send over channel").unwrap();
		});
		
		glib::spawn_future_local(clone!(
			#[weak]
			window,
			async move {
				if let Ok(response) = receiver.recv().await {
					if let Err(e) = response {
						error_dialog(window.clone(), "Command failed", e.to_string()).await;
					} else {
						enable_service_dialog(window.clone()).await;
					}
				}
			}
		));
	}
}

pub async fn group_dialog(window: ApplicationWindow) {
	let question_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.buttons(["No", "Yes"])
		.message("Must be in the group 'input'.")
		.detail("Do you want to be automatically added to it? (will ask for root)")
		.build();
	
	let answer = question_dialog.choose_future(Some(&window)).await.unwrap();

	if answer == 1 {
		let (sender, receiver) = async_channel::bounded::<anyhow::Result<()>>(1);
		runtime().spawn(async move {
			sender.send(unix::add_user_to_group("input").await).await.context("could not send over channel").unwrap();
		});

		glib::spawn_future_local(clone!(
			#[weak]
			window,
			async move {
				match receiver.recv().await {
					Ok(Err(e)) => error_dialog(window.clone(), "Command failed", e.to_string()).await,
					Ok(Ok(_)) => reboot_dialog(&window).await,
					_ => {}
					// Err(_) => panic!("could not recv msg from channel"),
				}
			}
		));
	}
}

pub fn sequence_dialog(window: &ApplicationWindow, config: Arc<Mutex<Config>>) {
	let dialog = gtk::Window::builder()
		.transient_for(window)
		.modal(true)
		.title("Key sequence")
		.default_width(500)
		.default_height(500)
		.build();
	
	let vbox = gtk::Box::builder()
		.orientation(gtk::Orientation::Vertical)
		.margin_top(24)
		.margin_bottom(24)
		.margin_start(24)
		.margin_end(24)
		.halign(gtk::Align::Fill)
		.valign(gtk::Align::Fill)
		.spacing(12)
		.hexpand(true)
		.vexpand(true)
		.build();
	
	let scrollable = gtk::ScrolledWindow::builder()
		.vexpand(true)
		.hexpand(true)
		.build();
	
	let debounce_id: Arc<Mutex<Option<glib::SourceId>>> = Arc::new(Mutex::new(None));
	let tag_table = gtk::TextTagTable::new();
	let str_tag = gtk::TextTag::builder().name("string").foreground("green").build();
	let key_tag = gtk::TextTag::builder().name("keycode").foreground("cyan").build();
	let error_tag = gtk::TextTag::builder().name("invalid_keycode").foreground("red").build();
	let action_tag = gtk::TextTag::builder().name("action").foreground("white").build();
	tag_table.add(&str_tag);
	tag_table.add(&key_tag);
	tag_table.add(&error_tag);
	tag_table.add(&action_tag);

	let buffer = gtk::TextBuffer::new(Some(&tag_table));
	let clone = buffer.clone();
	buffer.connect_changed(clone!(
		#[strong]
		debounce_id,
		#[weak]
		clone,
		move |_| {
			let mut lock = debounce_id.lock().unwrap();
			if lock.is_some() {
				lock.take().unwrap().remove();
			}
			drop(lock);

			let debounce_clone = debounce_id.clone();
			let id = glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
				key_parser::syntax_highlighting(&clone);
				*debounce_clone.lock().unwrap() = None;
				return glib::ControlFlow::Break;
			});
			*debounce_id.lock().unwrap() = Some(id);
			// events::syntax_highlighting(&debounce_id, clone);
		}
	));

	let entry = gtk::TextView::builder().buffer(&buffer).monospace(true).build();
	{
		let config = config.lock().unwrap();
		entry.buffer().set_text(&config.keyboard.raw_sequence);
	}
	dialog.set_child(Some(&vbox));
	scrollable.set_child(Some(&entry));
	vbox.append(&scrollable);
	
	let button_grid = gtk::Grid::builder()
		.row_spacing(6)
		.column_spacing(6)
		.column_homogeneous(true)
		.row_homogeneous(true)
		.build();
	
	let cancel_button = gtk::Button::with_label("Cancel");
	cancel_button.connect_clicked(clone!(
		#[weak]
		dialog,
		move |_| {
			dialog.close();
		}
	));
	
	let ok_button = gtk::Button::with_label("Ok");
	ok_button.add_css_class("suggested-action");
	ok_button.connect_clicked(clone!(
		#[weak]
		config,
		#[weak]
		entry,
		#[weak]
		window,
		#[weak]
		dialog,
		move |_| {
			let mut config = config.lock().unwrap();
			let buffer = entry.buffer();
			let (start, end) = buffer.bounds();
			let text = buffer.text(&start, &end, true).to_string();

			config.keyboard.raw_sequence = text.clone();
			config.keyboard.sequence = match key_parser::parse(text) {
				Ok(o) => o,
				Err(e) => {
					glib::MainContext::default().spawn_local(error_dialog(window.clone(), "Error: parse_sequence", e.to_string()));
					return;
				}
			};
			tracing::debug!(?config);
			dialog.close();
		}
	));
	let key_controller = gtk::EventControllerKey::new();
	key_controller.connect_key_pressed(clone!(
		#[weak]
		config,
		#[weak]
		entry,
		#[weak]
		window,
		#[weak]
		dialog,
		#[upgrade_or]
		glib::Propagation::Proceed,
		move |_, keyval, _keycode, state| {
			if keyval == gtk::gdk::Key::Return && state.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
				let mut config = config.lock().unwrap();
				let buffer = entry.buffer();
				let (start, end) = buffer.bounds();
				let text = buffer.text(&start, &end, true).to_string();

				config.keyboard.raw_sequence = text.clone();
				config.keyboard.sequence = match key_parser::parse(text) {
					Ok(o) => o,
					Err(e) => {
						glib::MainContext::default().spawn_local(error_dialog(window.clone(), "Error: parse_sequence", e.to_string()));
						return glib::Propagation::Stop;
					}
				};
				tracing::debug!(?config);
				dialog.close();
				
				return glib::Propagation::Stop;
			}

			glib::Propagation::Proceed
		}
	));
	entry.add_controller(key_controller);

	button_grid.attach(&cancel_button, 0, 0, 1, 1);
	button_grid.attach(&ok_button, 1, 0, 1, 1);

	vbox.append(&button_grid);

	dialog.present();
}

async fn reboot_dialog(window: &ApplicationWindow) {
	tracing::debug!("opening reboot dialog");
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message("Reboot")
		.detail("To apply the changes, you must reboot")
		.build();

	info_dialog.show(Some(window));
}

pub async fn short_duration_dialog(window: ApplicationWindow) {
	tracing::debug!("opening short duration dialog");
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message("Duration too short")
		.detail("With an interval of <25ms, your computer can have intense amounts of lag. Please set 'interval' higher.")
		.build();

	info_dialog.show(Some(&window));
}
