use anyhow::{anyhow, Context};
use gtk4 as gtk;
use gtk::{
	ApplicationWindow,
	glib::{self, clone},
};

use super::runtime;

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
			let status = tokio::process::Command::new("/usr/bin/systemctl")
				.args(["--user", "enable", "autoclickerd.service"])
				.status().await.context("could not run '/usr/bin/systemctl'")?;

			if !status.success() {
				sender.send(Err(anyhow!("systemctl failed, code: {}", status))).await.unwrap();
				return Err(anyhow!("systemctl failed, code: {}", status));
			}

			sender.send(Ok(())).await.unwrap();
			return Ok(());
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
			let status = tokio::process::Command::new("/usr/bin/systemctl")
				.args(["--user", "start", "autoclickerd.service"])
				.status().await.context("could not run '/usr/bin/systemctl'")?;

			if !status.success() {
				sender.send(Err(anyhow!("systemctl failed, code: {}", status))).await.unwrap();
				return Err(anyhow!("systemctl failed, code: {}", status));
			}

			sender.send(Ok(())).await.unwrap();
			return Ok(());
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
			let user = nix::unistd::User::from_uid(nix::unistd::geteuid()).context("from_uid failed")?.context("from_uid failed")?;
			let status = tokio::process::Command::new("/usr/bin/pkexec")
				.args(["sh", "-c", format!("/usr/bin/usermod -aG input '{}'", user.name).as_str()])
				.status().await.context("could not run '/usr/bin/pkexec'")?;

			if !status.success() {
				sender.send(Err(anyhow!("pkexec failed, code: {}", status))).await.unwrap();
				return Err(anyhow!("pkexec failed, code: {}", status));
			}
			
			sender.send(Ok(())).await.unwrap();
			return Ok(());
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
