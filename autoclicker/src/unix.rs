use anyhow::{Context, anyhow};

pub async fn start_systemd_service(service: &str) -> anyhow::Result<()> {
	let status = tokio::process::Command::new("/usr/bin/systemctl")
		.args(["--user", "start", service])
		.status().await.context("could not run '/usr/bin/systemctl'")?;

	if !status.success() {
		return Err(anyhow!("systemctl failed, code: {}", status));
	}
	
	return Ok(());
}

pub async fn enable_systemd_service(service: &str) -> anyhow::Result<()> {
	let status = tokio::process::Command::new("/usr/bin/systemctl")
		.args(["--user", "enable", service])
		.status().await.context("could not run '/usr/bin/systemctl'")?;

	if !status.success() {
		return Err(anyhow!("systemctl failed, code: {}", status));
	}
	
	return Ok(());
}

pub async fn add_user_to_group(group: &str) -> anyhow::Result<()> {
	let user = nix::unistd::User::from_uid(nix::unistd::geteuid()).context("from_uid failed")?.context("from_uid failed")?;
	let status = tokio::process::Command::new("/usr/bin/pkexec")
		.args(["sh", "-c", format!("/usr/bin/usermod -aG '{}' '{}'", group, user.name).as_str()])
		.status().await.context("could not run '/usr/bin/pkexec'")?;

	if !status.success() {
		return Err(anyhow!("pkexec failed, code: {}", status));
	}

	return Ok(());
}

pub fn is_user_in_group(wanted_group: &str) -> anyhow::Result<bool> {
	let groups = nix::unistd::getgroups().context("could not get groups")?;
	let mut in_group = false;
	for group in groups {
		let group = nix::unistd::Group::from_gid(group).context("could not get group from gid")?.context("group does not exist")?;
		if group.name == wanted_group {
			in_group = true;
			break;
		}
	}
	
	return Ok(in_group);
}
