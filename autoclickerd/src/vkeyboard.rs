use anyhow::Context;
use evdev_rs::enums::{int_to_ev_key, BusType, EventCode, EV_KEY};
use evdev_rs::{DeviceWrapper, UInputDevice, UninitDevice};

use crate::vdevice::*;

#[allow(unused)]
pub struct Keyboard {
	keyboard: UninitDevice,
	input: UInputDevice
}

impl Keyboard {
	pub fn new() -> anyhow::Result<Self> {
		let keyboard = UninitDevice::new().context("could not create keyboard")?;
		keyboard.set_name("autoclicker virtual keyboard");
		keyboard.set_bustype(BusType::BUS_USB as u16);
		keyboard.set_vendor_id(0xabcd);
		keyboard.set_product_id(0xefef);
		
		for key in EV_KEY::KEY_ESC as u32..EV_KEY::KEY_MAX as u32 {
			if let Some(key) = int_to_ev_key(key) {
				keyboard.enable(EventCode::EV_KEY(key)).context("could not enable keyboard key")?;
			}
		}
		
		let input = UInputDevice::create_from_device(&keyboard).context("could not create input device")?;
		
		return Ok(Self {
			keyboard,
			input,
		});
	}

	pub fn release_keyboard_button(&self, key: EV_KEY) -> anyhow::Result<()> {
		self.send_event(EventCode::EV_KEY(key), 0)?;
		self.send_sync()?;
		
		return Ok(());
	}
	
	pub fn press_keyboard_button(&self, key: EV_KEY) -> anyhow::Result<()> {
		self.send_event(EventCode::EV_KEY(key), 1)?;
		self.send_sync()?;

		return Ok(());
	}
	
	pub fn click_keyboard_button(&self, key: EV_KEY) -> anyhow::Result<()> {
		self.press_keyboard_button(key)?;
		self.release_keyboard_button(key)?;
		
		return Ok(());
	}
}

impl VirtualDevice for Keyboard {
	fn get_input(&self) -> &UInputDevice {
		return &self.input;
	}
}