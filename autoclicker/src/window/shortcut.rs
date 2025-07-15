use gtk4::{self as gtk, glib};

pub fn add_shortcut<F: Fn() + 'static>(controller: &gtk::ShortcutController, shortcut: &str, action: F) {
	let trigger = gtk::ShortcutTrigger::parse_string(shortcut).unwrap();
	let action = gtk::CallbackAction::new(move |_, _| {
		action();
		return glib::Propagation::Stop;
	});
	controller.add_shortcut(gtk::Shortcut::new(Some(trigger), Some(action)));
}
