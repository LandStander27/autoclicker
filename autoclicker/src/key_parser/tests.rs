#[cfg(test)]
use super::*;

#[test]
fn test_string_parsing() {
	let data = "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}, newline:\\nescaped whitespace: \\    abc\"";
	let result = strings::parse_string(data);
	assert_eq!(
		result,
		Ok(("", "tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂, newline:\nescaped whitespace: abc".into()))
	);

	assert!(strings::parse_string("\"this is another test\"").is_ok());
	assert!(strings::parse_string("\"this is another test").is_err());
	assert!(strings::parse_string("this is another test\"").is_err());
	assert!(strings::parse_string("\"this \\h is another test\"").is_err());
}

#[test]
fn test_keyword_parsing() {
	assert_eq!(key("Space Tab").unwrap(), (" Tab", "Space".into()));
}

#[test]
fn test_func_parsing() {
	assert!(func("delay(1, 2)").is_ok());
	assert!(parse("delay(1, 2)".into()).is_err());
	assert!(func("delay(1)").is_ok());
	assert!(func("press(Space)").is_ok());
	assert!(parse("not_a_func(Space)".into()).is_err());
}

#[test]
fn test_generic_parsing() {
	assert!(parse("Space Tab \"this is another test\"".into()).is_ok());
	assert!(parse("Space Tab \"this is another test".into()).is_err());
	assert!(parse("Space Tab this is another test\"".into()).is_err());

	assert!(parse("Space Tab move delay(2)".into()).is_ok());
	assert!(parse("Space Tab move dely".into()).is_err());
}

#[test]
fn test_number_parsing() {
	assert!(parse_number("234").is_ok());
	assert!(parse_number("-234").is_ok());
	assert!(parse_number("a234").is_err());
	assert_eq!(parse_number("-23a4"), Ok(("a4", -23)));
}

#[test]
fn test_actions_parsing() {
	let vec = parse("Space Tab \"a\\n \\t \\\\ \\\"test\" delay(2)".into()).unwrap();
	assert_eq!(
		vec,
		[
			Actions::PressAndRelease("KEY_SPACE".into()),
			Actions::PressAndRelease("KEY_TAB".into()),
			Actions::PressAndRelease("KEY_A".into()),
			Actions::PressAndRelease("KEY_ENTER".into()),
			Actions::PressAndRelease("KEY_SPACE".into()),
			Actions::PressAndRelease("KEY_TAB".into()),
			Actions::PressAndRelease("KEY_SPACE".into()),
			Actions::PressAndRelease("KEY_BACKSLASH".into()),
			Actions::PressAndRelease("KEY_SPACE".into()),
			Actions::Press("KEY_LEFTSHIFT".into()),
			Actions::Press("KEY_APOSTROPHE".into()),
			Actions::Release("KEY_LEFTSHIFT".into()),
			Actions::Release("KEY_APOSTROPHE".into()),
			Actions::PressAndRelease("KEY_T".into()),
			Actions::PressAndRelease("KEY_E".into()),
			Actions::PressAndRelease("KEY_S".into()),
			Actions::PressAndRelease("KEY_T".into()),
			Actions::Delay(2),
		]
	);
}
