use anyhow::Context;
use gtk4 as gtk;
use gtk::{
	ApplicationWindow,
	glib::{self, clone},
};

use super::runtime;
use crate::unix;

pub async fn error_dialog(window: gtk::ApplicationWindow, title: &str, msg: String) {
	tracing::debug!("opening error dialog");
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message(title)
		.detail(msg)
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
