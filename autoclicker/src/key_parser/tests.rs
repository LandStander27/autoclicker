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
	parse("delay(1, 2)".into()).unwrap();
	// assert_eq!();
}

#[test]
fn test_generic_parsing() {
	assert!(parse("Space Tab \"this is another test\"".into()).is_ok());
	assert!(parse("Space Tab \"this is another test".into()).is_err());
	assert!(parse("Space Tab this is another test\"".into()).is_err());

	assert!(parse("Space Tab move delay(2)".into()).is_ok());
	assert!(parse("Space Tab move dely".into()).is_err());
}
