use anyhow::{anyhow, Context};
use gtk4 as gtk;
use gtk::{
	ApplicationWindow,
	glib::{self, clone},
};

use super::runtime;

pub async fn error_dialog(window: gtk::ApplicationWindow, title: &str, msg: String) {
	let info_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.message(title)
		.detail(msg)
		.build();

	info_dialog.show(Some(&window));
}

pub async fn group_dialog(window: ApplicationWindow) {
	let question_dialog = gtk::AlertDialog::builder()
		.modal(true)
		.buttons(["Cancel", "Ok"])
		.message("Must be in the group 'input'.")
		.detail("Do you want to be automatically added to it? (will ask for root)")
		.build();
	
	let answer = question_dialog.choose_future(Some(&window)).await.unwrap();

	if answer == 1 {
		let (sender, receiver) = async_channel::bounded::<anyhow::Result<()>>(1);
		runtime().spawn(clone!(
			#[strong]
			sender,
			async move {
				let user = nix::unistd::User::from_uid(nix::unistd::geteuid()).context("from_uid failed")?.context("from_uid failed")?;
				let status = tokio::process::Command::new("/usr/bin/pkexec")
					.args(["sh", "-c", format!("/usr/bin/usermod -aG input '{}'", user.name).as_str()])
					.status().await.context("could not run '/usr/bin/pkexec'")?;

				if !status.success() {
					return Err(anyhow!("pkexec failed, code: {}", status));
				}
				
				sender.send(Ok(())).await.unwrap();
				return Ok(());
			}
		));

		glib::spawn_future_local(clone!(
			#[weak]
			window,
			async move {
				while let Ok(response) = receiver.recv().await {
					if let Err(e) = response {
						error_dialog(window.clone(), "Command failed", e.to_string()).await;
					}
				}
			}
		));
	}
}