use anyhow::Context;
use evdev_rs::enums::{EV_SYN, EventCode};
use evdev_rs::{InputEvent, TimeVal, UInputDevice};

pub trait VirtualDevice {
	fn get_input(&self) -> &UInputDevice;
	fn send_event(&self, ev: EventCode, value: i32) -> anyhow::Result<()> {
		self.get_input()
			.write_event(&InputEvent {
				event_code: ev,
				value,
				time: self.get_time()?,
			})
			.context("could not write event to input device")?;

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
