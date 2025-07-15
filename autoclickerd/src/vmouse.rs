use anyhow::Context;
use evdev_rs::enums::{BusType, EventCode, EV_KEY, EV_REL, EV_SYN};
use evdev_rs::{DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice};

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
		let mouse = UninitDevice::new().context("could not create device")?;
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

	pub fn move_mouse(&self, x: Option<i32>, y: Option<i32>) -> anyhow::Result<()> {
		if x.is_some() {
			self.send_event(EventCode::EV_REL(EV_REL::REL_X), i32::MIN)?;
		}
		if y.is_some() {
			self.send_event(EventCode::EV_REL(EV_REL::REL_Y), i32::MIN)?;
		}
		self.send_sync()?;

		if let Some(x) = x {
			self.send_event(EventCode::EV_REL(EV_REL::REL_X), x)?;
		}
		if let Some(y) = y {
			self.send_event(EventCode::EV_REL(EV_REL::REL_Y), y)?;
		}
		self.send_sync()?;
		
		return Ok(());
	}

	pub fn click_button(&self, button: MouseButton) -> anyhow::Result<()> {
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

	fn send_event(&self, ev: EventCode, value: i32) -> anyhow::Result<()> {
		self.input.write_event(&InputEvent {
			event_code: ev,
			value,
			time: self.get_time()?,
		}).context("could not write event to input device")?;
		
		return Ok(());
	}

	fn get_time(&self) -> anyhow::Result<TimeVal> {
		return TimeVal::try_from(std::time::SystemTime::now()).context("could not convert SystemTime to TimeVal");
	}
	
	fn send_sync(&self) -> anyhow::Result<()> {
		self.send_event(EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0)?;
		return Ok(());
	}
}
