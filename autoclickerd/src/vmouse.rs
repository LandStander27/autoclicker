use anyhow::Context;
use evdev_rs::enums::{BusType, EventCode, EV_KEY, EV_REL, EV_SYN};
use evdev_rs::{DeviceWrapper, UInputDevice, UninitDevice};

use crate::vdevice::*;

pub enum MouseButton {
	Left,
	Right,
	Middle,
}

#[allow(unused)]
pub struct Mouse {
	mouse: UninitDevice,
	input: UInputDevice
}

impl Mouse {
	pub fn new() -> anyhow::Result<Self> {
		let mouse = UninitDevice::new().context("could not create mouse")?;
		mouse.set_name("autoclicker virtual mouse");
		mouse.set_bustype(BusType::BUS_USB as u16);
		mouse.set_vendor_id(0xabcd);
		mouse.set_product_id(0xefef);
		
		mouse.enable(EventCode::EV_KEY(EV_KEY::BTN_LEFT)).context("could not enable left mouse key")?;
		mouse.enable(EventCode::EV_KEY(EV_KEY::BTN_MIDDLE)).context("could not enable middle mouse key")?;
		mouse.enable(EventCode::EV_KEY(EV_KEY::BTN_RIGHT)).context("could not enable right mouse key")?;
		
		mouse.enable(EventCode::EV_REL(EV_REL::REL_X)).context("could not enable rel_x")?;
		mouse.enable(EventCode::EV_REL(EV_REL::REL_Y)).context("could not enable rel_y")?;
		
		mouse.enable(EventCode::EV_SYN(EV_SYN::SYN_REPORT)).context("could not enable SYN_REPORT")?;
		
		let input = UInputDevice::create_from_device(&mouse).context("could not create input device")?;
		
		return Ok(Self {
			mouse,
			input,
		});
	}
	
	#[inline]
	pub fn move_mouse_relative(&self, x: Option<i32>, y: Option<i32>) -> anyhow::Result<()> {
		if let Some(x) = x {
			self.send_event(EventCode::EV_REL(EV_REL::REL_X), x)?;
		}
		if let Some(y) = y {
			self.send_event(EventCode::EV_REL(EV_REL::REL_Y), y)?;
		}
		self.send_sync()?;
		
		return Ok(());
	}

	#[inline]
	pub fn move_mouse(&self, x: Option<i32>, y: Option<i32>) -> anyhow::Result<()> {
		self.move_mouse_relative(Some(i32::MIN), Some(i32::MIN))?;
		self.move_mouse_relative(x, y)?;

		return Ok(());
	}

	#[inline]
	pub fn click_mouse_button(&self, button: MouseButton) -> anyhow::Result<()> {
		match button {
			MouseButton::Left => {
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_LEFT), 1)?;
				self.send_sync()?;
				
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_LEFT), 0)?;
				self.send_sync()?;
			}
			
			MouseButton::Right => {
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_RIGHT), 1)?;
				self.send_sync()?;
				
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_RIGHT), 0)?;
				self.send_sync()?;
			}
			
			MouseButton::Middle => {
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_MIDDLE), 1)?;
				self.send_sync()?;
				
				self.send_event(EventCode::EV_KEY(EV_KEY::BTN_MIDDLE), 0)?;
				self.send_sync()?;
			}
		}

		return Ok(());
	}
}

impl VirtualDevice for Mouse {
	fn get_input(&self) -> &UInputDevice {
		return &self.input;
	}
}