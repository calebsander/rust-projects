use std::char;
use std::collections::HashMap;
use std::mem;
use std::str::Chars;

const UNICODE_HEX_LENGTH: usize = 4;

#[derive(Debug)]
pub enum JSONValue {
	Null,
	Boolean(bool),
	Number(f64),
	String(String),
	Array(Vec<JSONValue>),
	Object(HashMap<String, JSONValue>),
}

struct ParsingArrayState {
		array: Vec<JSONValue>,
		read_comma: bool,
}
struct ParsingObjectState {
	object: HashMap<String, JSONValue>,
	field: Option<String>,
	read_comma: bool,
}
enum ParseState {
	ParsingArray(ParsingArrayState),
	ParsingObject(ParsingObjectState),
}

struct StrPosition<'a> {
	index: usize,
	chars: Chars<'a>,
}

impl Iterator for StrPosition<'_> {
	type Item = char;

	fn next(&mut self) -> Option<char> {
		let original_len = self.chars.as_str().len();
		let result = self.chars.next();
		self.index += original_len - self.chars.as_str().len();
		result
	}
}

fn parse_string(json: &str, pos: &mut StrPosition) -> Result<String, &'static str> {
	let mut start_pos = pos.index;
	let mut result = String::new();
	while let Some(c) = pos.next() {
		match c {
			'\\' => {
				result.push_str(&json[start_pos..(pos.index - 1)]);
				start_pos = pos.index + 1;
				let c = pos.next().ok_or("Missing character after escape")?;
				match c {
					'"' | '\\' | '/' => result.push(c),
					'b' => result.push('\x08'),
					'f' => result.push('\x0C'),
					'n' => result.push('\n'),
					't' => result.push('\t'),
					'u' => {
						let mut code_point = 0;
						for _ in 0..UNICODE_HEX_LENGTH {
							let hex_digit = pos.next().and_then(|c| c.to_digit(16))
								.ok_or("Invalid unicode escape")?;
							code_point = code_point << 4 | hex_digit;
						}
						result.push(char::from_u32(code_point).ok_or("Invalid unicode escape")?);
						start_pos += UNICODE_HEX_LENGTH;
					}
					_ => return Err("Invalid escape sequence"),
				}
			},
			'"' => {
				result.push_str(&json[start_pos..(pos.index - 1)]);
				return Ok(result);
			},
			_ => {},
		}
	}
	Err("Missing string terminator")
}

fn skip_whitespace(pos: &mut StrPosition) -> Result<char, &'static str> {
	for c in pos {
		if !c.is_whitespace() { return Ok(c) }
	}
	Err("Unexpected end of JSON")
}

fn is_number_char(c: char) -> bool {
	c == '+' || c == '-' || ('0' <= c && c <= '9') ||
	c == '.' || c == 'E' || c == 'e'
}

fn combine(state_stack: &mut Vec<ParseState>, value: JSONValue) -> Option<JSONValue> {
	use ParseState::*;

	match state_stack.last_mut() {
		Some(ParsingArray(state)) => {
			state.array.push(value);
			state.read_comma = false;
			None
		},
		Some(ParsingObject(state)) => {
			state.object.insert(state.field.take().unwrap(), value);
			state.read_comma = false;
			None
		},
		None => Some(value),
	}
}

fn parse_value(json: &str, c: char, pos: &mut StrPosition, state_stack: &mut Vec<ParseState>)
	-> Result<Option<JSONValue>, &'static str>
{
	use JSONValue::*;
	use ParseState::*;

	match c {
		'n' => {
			if pos.take(3).ne("ull".chars()) { return Err("Expected null") }
			Ok(combine(state_stack, Null))
		},
		'f' => {
			if pos.take(4).ne("alse".chars()) { return Err("Expected false") }
			Ok(combine(state_stack, Boolean(false)))
		},
		't' => {
			if pos.take(3).ne("rue".chars()) { return Err("Expected true") }
			Ok(combine(state_stack, Boolean(true)))
		},
		'"' => parse_string(json, pos)
			.map(|string| combine(state_stack, String(string))),
		'[' => {
			state_stack.push(ParsingArray(ParsingArrayState {
				array: vec![],
				read_comma: true,
			}));
			Ok(None)
		},
		'{' => {
			state_stack.push(ParsingObject(ParsingObjectState {
				object: HashMap::new(),
				field: None,
				read_comma: true,
			}));
			Ok(None)
		}
		_ => {
			if !is_number_char(c) { return Err("Expected a JSON value") }
			let number_start_index = pos.index - 1;
			loop {
				match json[pos.index..].chars().next() {
					Some(c) if is_number_char(c) => pos.next(),
					_ => break,
				};
			}
			match json[number_start_index..pos.index].parse() {
				Ok(number) => Ok(combine(state_stack, Number(number))),
				Err(_) => Err("Invalid number")
			}
		},
	}
}

pub fn from_json(json: &str) -> Result<JSONValue, &'static str> {
	use JSONValue::*;
	use ParseState::*;

	let mut state_stack = vec![];
	let mut pos = StrPosition { index: 0, chars: json.chars() };
	let value = loop {
		let c = skip_whitespace(&mut pos)?;
		match state_stack.last_mut() {
			Some(ParsingArray(state)) => match c {
				',' => {
					if state.read_comma { return Err("Expected ] or value") }
					state.read_comma = true;
				},
				']' => {
					if state.read_comma && !state.array.is_empty() {
						return Err("Expected value")
					}
					let array = mem::replace(&mut state.array, vec![]);
					state_stack.pop();
					if let Some(value) = combine(&mut state_stack, Array(array)) {
						break value
					}
				},
				_ => {
					if !state.read_comma { return Err("Expected ','") }
					if let Some(value) = parse_value(json, c, &mut pos, &mut state_stack)? {
						break value
					}
				},
			},
			Some(ParsingObject(state)) =>
				if state.field.is_some() {
					if let Some(value) = parse_value(json, c, &mut pos, &mut state_stack)? {
						break value
					}
				}
				else {
					match c {
						'"' => {
							if !state.read_comma { return Err("Expected ','") }
							state.field = Some(parse_string(json, &mut pos)?);
							if skip_whitespace(&mut pos)? != ':' { return Err("Expected ':'") }
						},
						',' => {
							if state.read_comma { return Err("Expected '\"' or '}'") }
							state.read_comma = true;
						},
						'}' => {
							if state.read_comma && !state.object.is_empty() {
								return Err("Expected value")
							}
							let object = mem::replace(&mut state.object, HashMap::new());
							state_stack.pop();
							if let Some(value) = combine(&mut state_stack, Object(object)) {
								break value
							}
						},
						_ => return Err("Expected '\"', ',', or '}'"),
					}
				},
			_ =>
				if let Some(value) = parse_value(json, c, &mut pos, &mut state_stack)? {
					break value
				},
		}
	};
	if skip_whitespace(&mut pos).is_ok() { return Err("Expected end of JSON") }
	Ok(value)
}

fn main() {
	println!("{:?}", from_json("[1, \"abc\", {}, true, { \"\\u00e9\" :\"\", \"abc\": \"def\" }, null, [[]]]"));
}