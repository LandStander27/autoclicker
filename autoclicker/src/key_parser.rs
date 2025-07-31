use nom::{
	branch::alt,
	bytes::complete::{tag, take_while1},
	character::complete::{alpha1, alphanumeric1, char, multispace0, one_of},
	combinator::{map, recognize, opt, cut},
	multi::{many0, many1, separated_list0},
	sequence::{delimited, pair, preceded, terminated},
	IResult,
	Parser,
	error::{context, ParseError},
	Offset,
};
use nom_language::error::{VerboseError, VerboseErrorKind};

use common::prelude::*;
use tracing::{error, info};
use anyhow::anyhow;
use std::fmt::Write;
use crate::keycodes;

pub mod strings;
mod tests;

type ParseResult<'a, I, O> = IResult<I, O, VerboseError<&'a str>>;

#[derive(Debug, PartialEq)]
enum Literal {
	String(String),
	Number(i64),
}

#[derive(Debug)]
enum Token<'a> {
	Sequence(String),
	Key(String),
	// Ident(String),
	Action((String, Vec<Literal>)),
	Unknown(&'a str),
}

fn parse_ident(input: &str) -> ParseResult<&str, String> {
	let res = recognize(pair(
		alt((alpha1, tag("_"))),
		many0(alt((alphanumeric1, tag("_"))))
	)).parse(input)?;
	
	return Ok((res.0, res.1.into()));
}

fn parse_number(input: &str) -> ParseResult<&str, i64> {
	let res = recognize(
		preceded(opt(char('-')), many1(
			terminated(one_of("0123456789"), many0(char('_')))
		))
	).parse(input)?;

	let num: i64 = match res.1.parse() {
		Ok(o) => o,
		Err(_) => {
			let mut err = VerboseError::from_error_kind(res.1, nom::error::ErrorKind::Fail);
			err.errors.push((res.1, VerboseErrorKind::Context("invalid i64")));
			return Err(nom::Err::Error(err));
		}
	};

	return Ok((res.0, num));
}

fn func(input: &str) -> ParseResult<&str, (String, Vec<Literal>)> {
	let res = recognize(pair(
		alt((alpha1, tag("_"))),
		many0(alt((alphanumeric1, tag("_"))))
	)).parse(input)?;
	
	let ident = res.1;
	if !["press", "release", "delay"].contains(&ident) {
		let mut err = VerboseError::from_error_kind(res.1, nom::error::ErrorKind::Fail);
		err.errors.push((res.1, VerboseErrorKind::Context("unknown function")));
		return Err(nom::Err::Error(err));
	}

	let res = cut(delimited(
		context("expected (", char('(')),
		separated_list0(char(','), delimited(
			multispace0,
			alt((
				map(parse_ident, Literal::String),
				map(parse_number, Literal::Number),
			)),
			multispace0
		)),
		context("expected ')'", char(')'))
	)).parse(res.0)?;

	return Ok((
		res.0,
		(ident.to_string(), res.1),
	));
}

fn key(input: &str) -> ParseResult<&str, String> {
	let res = recognize(pair(
		alt((alpha1, tag("_"))),
		many0(alt((alphanumeric1, tag("_"))))
	)).parse(input)?;

	if keycodes::key_exists(res.1) {
		return Ok((res.0, res.1.into()));
	}
	
	let mut err = VerboseError::from_error_kind(res.1, nom::error::ErrorKind::Fail);
	err.errors.push((res.1, VerboseErrorKind::Context("unknown key")));
	return Err(nom::Err::Error(err));
}

fn convert_error(input: &String, err: VerboseError<&str>) -> String {
	let mut result = String::new();
	
	for (substring, kind) in err.errors.iter() {
		let offset = input.offset(substring);
		
		if input.is_empty() {
			match kind {
				VerboseErrorKind::Char(c) => write!(&mut result, "error: expected '{c}', got empty input\n\n"),
				VerboseErrorKind::Context(ctx) => write!(&mut result, "error: in {ctx}, got empty input\n\n"),
				VerboseErrorKind::Nom(e) => write!(&mut result, "error: in {e:?}, got empty input\n\n"),
			}.unwrap();

			continue;
		}

		let prefix = &input.as_bytes()[..offset];

		let line_number = prefix.iter().filter(|&&b| b == b'\n').count() + 1;
		let line_begin = prefix
			.iter()
			.rev()
			.position(|&b| b == b'\n')
			.map(|pos| offset - pos)
			.unwrap_or(0);
		
		let line = input[line_begin..]
			.lines()
			.next()
			.unwrap_or(&input[line_begin..])
			.trim_end();
		
		let column_number = line.offset(substring) + 1;

		match kind {
			VerboseErrorKind::Char(c) => {
				if let Some(actual) = substring.chars().next() {
					write!(&mut result, "error: expected '{c}', found {actual}\nline:column {line_number}:{column}\n{line}\n\n", column = column_number)
				} else {
					write!(&mut result, "error: expected '{c}', got end of input\nline:column {line_number}:{column}\n{line}\n\n", column = column_number)
				}
			}
			VerboseErrorKind::Context(ctx) => {
				write!(&mut result, "error: {ctx}\nline:column {line_number}:{column}\n{line}\n\n", column = column_number)
			}
			VerboseErrorKind::Nom(err) => {
				write!(&mut result, "error: {err:?}\nline:column {line_number}:{column}\n{line}\n\n", column = column_number)
			}
			// VerboseErrorKind::Char(c) => {
			// 	if let Some(actual) = substring.chars().next() {
			// 		write!(&mut result, "error: expected '{c}', found {actual}\n   --> line {line_number}\n     {line}\n{caret:>column$}\n\n", caret = "^", column = column_number + 5)
			// 	} else {
			// 		write!(&mut result, "error: expected '{c}', got end of input\n   --> line {line_number}\n     {line}\n{caret:>column$}\n\n", caret = "^", column = column_number + 5)
			// 	}
			// }
			// VerboseErrorKind::Context(ctx) => {
			// 	write!(&mut result, "error: {ctx}\n   --> line {line_number}\n     {line}\n{caret:>column$}\n\n", caret = "^", column = column_number + 5)
			// }
			// VerboseErrorKind::Nom(err) => {
			// 	write!(&mut result, "error: {err:?}\n   --> line {line_number}\n     {line}\n{caret:>column$}\n\n", caret = "^", column = column_number + 5)
			// }
		}.unwrap();
	}

	return result;
}

pub(crate) fn parse(input: String) -> anyhow::Result<Vec<Actions>> {
	let start = std::time::Instant::now();

	let mut rest: &str = input.as_str();
	let mut actions: Vec<Actions> = Vec::new();

	loop {
		let res = preceded(multispace0, alt((
			map(strings::parse_string, Token::Sequence),
			map(func, Token::Action),
			map(key, Token::Key),
			map(take_while1(|c: char| !c.is_whitespace()), Token::Unknown),
		))).parse(rest).map_err(|e| {
			match e {
				nom::Err::Error(ref e) | nom::Err::Failure(ref e) => {
					let s = convert_error(&input, e.clone());
					error!(s);
					return anyhow!("{s}");
				}
				_ => {}
			}

			error!(?e);
			return anyhow!("{e}").context("parse error");
		})?;

		rest = res.0;

		#[cfg(debug_assertions)]
		dbg!(&res.1);

		match res.1 {
			Token::Unknown(token) => {
				error!("unknown token: {token}");
				return Err(anyhow!("unknown token: {token}"));
			}
			Token::Sequence(seq) => {
				macro_rules! press_and_release {
					($key:expr) => {{
						actions.push(Actions::PressAndRelease($key.into()));
					}};
				}

				macro_rules! holding_shift {
					($key:expr) => {{
						actions.push(Actions::Press("KEY_LEFTSHIFT".into()));
						actions.push(Actions::Press($key.into()));

						actions.push(Actions::Release("KEY_LEFTSHIFT".into()));
						actions.push(Actions::Release($key.into()));
					}};
				}

				macro_rules! generate_match {
					($c:ident, [ $( ( $key:literal, $without:literal $(, $with:literal)? ) ),* ]) => {{
						match $c {
							'A'..='Z' | 'a'..='z' | '0'..='9' => {
								let mut s = String::new();
								s.push_str("KEY_");
								s.push($c.to_ascii_uppercase());
								actions.push(Actions::PressAndRelease(s));
							}
							$(
								#[allow(unreachable_patterns)] $without => press_and_release!(concat!("KEY_", $key)),
								$($with => holding_shift!(concat!("KEY_", $key)),)?
							)*
							_ => continue
						}
					}};
				}

				for c in seq.chars() {
					generate_match!(c, [
						("MINUS", '-', '_'),
						("EQUAL", '=', '+'),
						("TAB", '\t'),
						("LEFTBRACE", '[', '{'),
						("RIGHTBRACE", ']', '}'),
						("ENTER", '\n'),
						("SEMICOLON", ';', ':'),
						("APOSTROPHE", '\'', '\"'),
						("GRAVE", '`', '~'),
						("BACKSLASH", '\\', '|'),
						("COMMA", ',', '<'),
						("DOT", '.', '>'),
						("SLASH", '/', '?'),
						("SPACE", ' '),
						
						("1", '\0', '!'),
						("2", '\0', '@'),
						("3", '\0', '#'),
						("4", '\0', '$'),
						("5", '\0', '%'),
						("6", '\0', '^'),
						("7", '\0', '&'),
						("8", '\0', '*'),
						("9", '\0', '('),
						("0", '\0', ')')
					]);
				}
			}
			Token::Key(kw) => {
				let mut s = String::new();
				s.push_str("KEY_");
				s.push_str(&kw.to_uppercase());
				actions.push(Actions::PressAndRelease(s));
			}
			Token::Action(action) => {
				let args = action.1;
				match action.0.as_str() {
					"delay" => {
						match args.as_slice() {
							[Literal::Number(num)] => {
								actions.push(Actions::Delay(*num));
							}
							_ => return Err(anyhow!("delay is defined as: `delay(number)`"))
						}
					}
					"press" => {
						match args.as_slice() {
							[Literal::String(key)] => {
								if !keycodes::key_exists(key) {
									return Err(anyhow!("invalid key: {key}"));
								}
								let mut s = String::new();
								s.push_str("KEY_");
								s.push_str(&key.to_uppercase());
								actions.push(Actions::Press(s));
							}
							_ => return Err(anyhow!("press is defined as: `press(key)`"))
						}
					}
					"release" => {
						match args.as_slice() {
							[Literal::String(key)] => {
								if !keycodes::key_exists(key) {
									return Err(anyhow!("invalid key: {key}"));
								}
								let mut s = String::new();
								s.push_str("KEY_");
								s.push_str(&key.to_uppercase());
								actions.push(Actions::Release(s));
							}
							_ => return Err(anyhow!("release is defined as: `release(key)`"))
						}
					}
					_ => unreachable!(),
				}
			}
		}

		if rest.is_empty() || rest.chars().all(|c| c.is_whitespace()) {
			break;
		}
	}
	
	info!("parsing done; took {}ms", start.elapsed().as_millis());
	return Ok(actions);
}

macro_rules! tag_buffer {
	($buffer:ident, $start:expr, $len:expr, $tag:expr) => {{
		let mut iter = $buffer.start_iter();
		iter.forward_chars($start as i32);
		let mut end_iter = iter;
		end_iter.forward_chars($len as i32);
		$buffer.apply_tag_by_name($tag, &iter, &end_iter);
	}};
}

pub(crate) fn syntax_highlighting(buffer: &gtk4::TextBuffer) {
	use gtk4 as gtk;
	use gtk::prelude::*;
	
	buffer.remove_all_tags(&buffer.start_iter(), &buffer.end_iter());
	let input = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).to_string();
	let mut offset: usize = 0;
	let mut rest: &str = input.as_str();

	fn inner(rest: &str) -> anyhow::Result<(&str, Token)> {
		let res = preceded(multispace0, alt((
			map(strings::parse_string, Token::Sequence),
			map(func, Token::Action),
			map(key, Token::Key),
			map(take_while1(|c: char| !c.is_whitespace()), Token::Unknown),
		))).parse(rest).map_err(|e| {
			error!(?e);
			return anyhow!("{e}").context("parse error");
		})?;

		return Ok(res);
	}

	loop {
		let res = if let Ok(o) = inner(rest) {
			o
		} else {
			break;
		};

		let len = rest.len() - res.0.len();
		rest = res.0;

		#[cfg(debug_assertions)]
		dbg!(&res.1);

		match res.1 {
			Token::Sequence(_) => {
				tag_buffer!(buffer, offset, len, "string");
			}
			Token::Key(_) => {
				tag_buffer!(buffer, offset, len, "keycode");
			}
			Token::Action(_) => {
				tag_buffer!(buffer, offset, len, "action");
			}
			_ => {}
			// Token::Unknown(_) => {
			// 	// tag_buffer!(buffer, offset, len, "invalid_keycode");
			// 	// error!("unknown token: {unk}");
			// }
		}
		offset += len;

		if rest.is_empty() || rest.chars().all(|c| c.is_whitespace()) {
			break;
		}
	}
}
