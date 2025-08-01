use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
	pub disable_window_controls: bool,
}