use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Actions {
	PressAndRelease(String),
	Press(String),
	Release(String),
	Delay(i64),
}
