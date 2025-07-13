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
pub struct MouseClick {
	pub button: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Error {
	pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Message {
	MouseClick(MouseClick),
	Error(Error),
}

impl Json for Message {}