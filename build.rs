fn main() {
	if let Err(e) = std::process::Command::new("../update_version.sh").spawn() {
		println!("{}", e);
		std::process::exit(1);
	}
}