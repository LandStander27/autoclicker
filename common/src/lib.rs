use serde::{Deserialize, Serialize};

pub mod prelude;

pub trait Json<T: for<'de> Deserialize<'de> + Serialize = Self> {
	fn decode<S: Into<String>>(json: S) -> Result<T, serde_json::Error> {
		return serde_json::from_str(&json.into());
	}
	
	fn encode(message: &T) -> Result<String, serde_json::Error> {
		return serde_json::to_string(message);
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopClicking {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepeatingMouseClick {
	pub button: String,
	pub typ: String,
	pub amount: u64,
	pub position: (Option<i32>, Option<i32>),
	pub interval: u64,
	// pub delay_until_first_click: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepeatingKeyboardClick {
	pub button: Vec<Vec<String>>,
	pub amount: u64,
	pub interval: u64,
	pub delay_before_repeat: u64,
	pub hold_duration: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MouseClick {
	pub button: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfirmResponse {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Error {
	pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Message {
	MouseClick(MouseClick),
	RepeatingMouseClick(RepeatingMouseClick),
	RepeatingKeyboardClick(RepeatingKeyboardClick),
	StopClicking(StopClicking),
	ConfirmResponse(ConfirmResponse),
	Error(Error),
}

impl Json for Message {}