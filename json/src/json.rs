use std::char;
use std::collections::HashMap;
use std::mem;

const UNICODE_HEX_LENGTH: usize = 4;

#[derive(Debug, Clone, PartialEq)]
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
enum ObjectState {
	BeforeField,
	BeforeValue(String),
	AfterValue,
}
struct ParsingObjectState {
	object: HashMap<String, JSONValue>,
	state: ObjectState,
}
enum ParseState {
	ParsingArray(ParsingArrayState),
	ParsingObject(ParsingObjectState),
}

struct StrPosition<'a> {
	string: &'a str,
	index: usize,
}

impl Iterator for StrPosition<'_> {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		let result = self.string.as_bytes().get(self.index).copied();
		self.index += 1;
		result
	}
}

fn parse_string(pos: &mut StrPosition) -> Result<String, &'static str> {
	let mut start_pos = pos.index;
	let mut result = String::new();
	while let Some(c) = pos.next() {
		if c == b'\\' {
			result.push_str(&pos.string[start_pos..(pos.index - 1)]);
			start_pos = pos.index + 1;
			let c = pos.next().ok_or("Missing character after escape")?;
			result.push(match c {
				b'"' | b'\\' | b'/' => c as char,
				b'b' => '\x08',
				b'f' => '\x0C',
				b'n' => '\n',
				b'r' => '\r',
				b't' => '\t',
				b'u' => {
					let mut code_point = 0;
					for _ in 0..UNICODE_HEX_LENGTH {
						let hex_digit = pos.next().and_then(|c| (c as char).to_digit(16))
							.ok_or("Invalid unicode escape")?;
						code_point = code_point << 4 | hex_digit;
					}
					start_pos += UNICODE_HEX_LENGTH;
					char::from_u32(code_point).ok_or("Invalid unicode escape")?
				},
				_ => return Err("Invalid escape sequence"),
			});
		}
		else if c == b'"' {
			result.push_str(&pos.string[start_pos..(pos.index - 1)]);
			return Ok(result);
		}
	}
	Err("Expected end of string")
}

fn skip_whitespace(pos: &mut StrPosition) -> Result<u8, &'static str> {
	for c in pos {
		match c {
			b' ' | b'\t' | b'\n' | b'\r' => continue,
			_ => return Ok(c),
		}
	}
	Err("Unexpected end of JSON")
}

fn is_number_char(c: u8) -> bool {
	c == b'+' || c == b'-' || b'0' <= c && c <= b'9' ||
	c == b'.' || c == b'E' || c == b'e'
}

fn combine(state_stack: &mut Vec<ParseState>, value: JSONValue) -> Option<JSONValue> {
	use ParseState::*;
	use ObjectState::*;

	match state_stack.last_mut() {
		Some(ParsingArray(state)) => {
			state.array.push(value);
			state.read_comma = false;
			None
		},
		Some(ParsingObject(ParsingObjectState { object, state })) => {
			let field = match mem::replace(state, AfterValue) {
				BeforeValue(field) => field,
				_ => unreachable!(),
			};
			object.insert(field, value);
			None
		},
		None => Some(value),
	}
}

fn parse_value(c: u8, pos: &mut StrPosition, state_stack: &mut Vec<ParseState>)
	-> Result<Option<JSONValue>, &'static str>
{
	use JSONValue::*;
	use ParseState::*;

	match c {
		b'n' => {
			if pos.take(3).ne("ull".bytes()) { return Err("Expected JSON value") }
			Ok(combine(state_stack, Null))
		},
		b'f' => {
			if pos.take(4).ne("alse".bytes()) { return Err("Expected JSON value") }
			Ok(combine(state_stack, Boolean(false)))
		},
		b't' => {
			if pos.take(3).ne("rue".bytes()) { return Err("Expected JSON value") }
			Ok(combine(state_stack, Boolean(true)))
		},
		b'"' => parse_string(pos).map(|string| combine(state_stack, String(string))),
		b'[' => {
			state_stack.push(ParsingArray(ParsingArrayState {
				array: vec![],
				read_comma: true,
			}));
			Ok(None)
		},
		b'{' => {
			state_stack.push(ParsingObject(ParsingObjectState {
				object: HashMap::new(),
				state: ObjectState::BeforeField,
			}));
			Ok(None)
		},
		_ => {
			if !is_number_char(c) { return Err("Expected JSON value") }
			let number_start_index = pos.index - 1;
			while let Some(c) = pos.next() {
				if !is_number_char(c) { break }
			}
			pos.index -= 1;
			match pos.string[number_start_index..pos.index].parse() {
				Ok(number) => Ok(combine(state_stack, Number(number))),
				Err(_) => Err("Invalid number")
			}
		},
	}
}

fn finish_array(state_stack: &mut Vec<ParseState>) -> Option<JSONValue> {
	match state_stack.pop() {
		Some(ParseState::ParsingArray(state)) =>
			combine(state_stack, JSONValue::Array(state.array)),
		_ => unreachable!(),
	}
}
fn finish_object(state_stack: &mut Vec<ParseState>) -> Option<JSONValue> {
	match state_stack.pop() {
		Some(ParseState::ParsingObject(state)) =>
			combine(state_stack, JSONValue::Object(state.object)),
		_ => unreachable!(),
	}
}

pub fn from_json(json: &str) -> Result<JSONValue, &'static str> {
	use ParseState::*;
	use ObjectState::*;

	let mut state_stack = vec![];
	let mut pos = StrPosition { string: json, index: 0 };
	let value = loop {
		let c = skip_whitespace(&mut pos)?;
		match state_stack.last_mut() {
			Some(ParsingArray(ParsingArrayState { array, read_comma })) => match c {
				b',' => {
					if *read_comma {
						return Err(
							if array.is_empty() { "Expected ']' or value" }
							else { "Expected value" }
						)
					}
					*read_comma = true;
				},
				b']' => {
					if *read_comma && !array.is_empty() {
						return Err("Expected value")
					}
					if let Some(value) = finish_array(&mut state_stack) {
						break value
					}
				},
				_ => {
					if !*read_comma { return Err("Expected ','") }
					if let Some(value) = parse_value(c, &mut pos, &mut state_stack)? {
						break value
					}
				},
			},
			Some(ParsingObject(ParsingObjectState { object, state })) => match state {
				BeforeField => match c {
					b'"' => {
						*state = BeforeValue(parse_string(&mut pos)?);
						if skip_whitespace(&mut pos)? != b':' { return Err("Expected ':'") }
					},
					b'}' => {
						if !object.is_empty() { return Err("Expected '\"'") }
						if let Some(value) = finish_object(&mut state_stack) {
							break value
						}
					},
					_ => return Err(
						if object.is_empty() { "Expected '\"' or '}'" }
						else { "Expected '\"'" }
					),
				},
				BeforeValue(_) =>
					if let Some(value) = parse_value(c, &mut pos, &mut state_stack)? {
						break value
					},
				AfterValue => match c {
					b',' => *state = BeforeField,
					b'}' =>
						if let Some(value) = finish_object(&mut state_stack) {
							break value
						},
					_ => return Err("Expected ',' or '}'"),
				},
			},
			_ =>
				if let Some(value) = parse_value(c, &mut pos, &mut state_stack)? {
					break value
				},
		}
	};
	if skip_whitespace(&mut pos).is_ok() { Err("Expected end of JSON") }
	else { Ok(value) }
}

fn from_json_recursive_pos(c: u8, pos: &mut StrPosition) -> Result<JSONValue, &'static str> {
	use JSONValue::*;
	use ObjectState::*;

	match c {
		b'n' => {
			if pos.take(3).ne("ull".bytes()) { return Err("Expected JSON value") }
			Ok(Null)
		},
		b'f' => {
			if pos.take(4).ne("alse".bytes()) { return Err("Expected JSON value") }
			Ok(Boolean(false))
		},
		b't' => {
			if pos.take(3).ne("rue".bytes()) { return Err("Expected JSON value") }
			Ok(Boolean(true))
		},
		b'"' => parse_string(pos).map(|string| String(string)),
		b'[' => {
			let mut array = vec![];
			let mut read_comma = true;
			loop {
				match skip_whitespace(pos)? {
					b',' => {
						if read_comma {
							return Err(
								if array.is_empty() { "Expected ']' or value" }
								else { "Expected value" }
							)
						}
						read_comma = true;
					},
					b']' => break
						if read_comma && !array.is_empty() { Err("Expected value") }
						else { Ok(Array(array)) },
					c => {
						if !read_comma { return Err("Expected ','") }
						array.push(from_json_recursive_pos(c, pos)?);
						read_comma = false;
					},
				}
			}
		},
		b'{' => {
			let mut object = HashMap::new();
			let mut state = BeforeField;
			loop {
				let c = skip_whitespace(pos)?;
				match state {
					BeforeField => match c {
						b'"' => {
							state = BeforeValue(parse_string(pos)?);
							if skip_whitespace(pos)? != b':' { return Err("Expected ':'") }
						},
						b'}' => {
							if !object.is_empty() { return Err("Expected '\"'") }
							break;
						},
						_ => return Err(
							if object.is_empty() { "Expected '\"' or '}'" }
							else { "Expected '\"'" }
						),
					},
					BeforeValue(field) => {
						object.insert(field, from_json_recursive_pos(c, pos)?);
						state = AfterValue;
					},
					AfterValue => match c {
						b',' => state = BeforeField,
						b'}' => break,
						_ => return Err("Expected ',' or '}'"),
					},
				}
			}
			Ok(Object(object))
		},
		_ => {
			if !is_number_char(c) { return Err("Expected JSON value") }
			let number_start_index = pos.index - 1;
			while let Some(c) = pos.next() {
				if !is_number_char(c) { break }
			}
			pos.index -= 1;
			match pos.string[number_start_index..pos.index].parse() {
				Ok(number) => Ok(Number(number)),
				Err(_) => Err("Invalid number")
			}
		},
	}
}
fn from_json_recursive(json: &str) -> Result<JSONValue, &'static str> {
	let mut pos = StrPosition { string: json, index: 0 };
	from_json_recursive_pos(skip_whitespace(&mut pos)?, &mut pos)
}

fn write_string(string: &str, json: &mut String) {
	json.push('"');
	let (mut start_index, mut index) = (0, 0);
	while let Some(c) = string.as_bytes().get(index).copied() {
		if c == b'"' || c == b'\\' {
			*json += &string[start_index..index];
			start_index = index + 1;
			json.push('\\');
			json.push(c as char);
		}
		index += 1;
	}
	*json += &string[start_index..];
	json.push('"');
}
fn write_json_value(value: &JSONValue, json: &mut String) {
	use JSONValue::*;

	match value {
		Null => *json += "null",
		Boolean(false) => *json += "false",
		Boolean(true) => *json += "true",
		Number(number) => *json += &*number.to_string(),
		String(string) => write_string(string, json),
		Array(array) => {
			json.push('[');
			for (i, value) in array.iter().enumerate() {
				if i > 0 { json.push(',') }
				write_json_value(value, json);
			}
			json.push(']');
		},
		Object(object) => {
			json.push('{');
			for (i, (key, value)) in object.iter().enumerate() {
				if i > 0 { json.push(',') }
				write_string(key, json);
				json.push(':');
				write_json_value(value, json);
			}
			json.push('}');
		}
	}
}
pub fn to_json(value: &JSONValue) -> String {
	let mut json = String::new();
	write_json_value(value, &mut json);
	json
}

#[cfg(test)]
mod tests {
	use super::*;
	use JSONValue::*;

	macro_rules! map(
		{ $($key:expr => $value:expr),* } => {
			vec![$(($key.into(), $value),)*].into_iter().collect()
		};
	);

	#[test]
	fn test_parse_null() {
		assert_eq!(from_json("null"), Ok(Null));
	}

	#[test]
	fn test_parse_boolean() {
		assert_eq!(from_json("true"), Ok(Boolean(true)));
		assert_eq!(from_json("false"), Ok(Boolean(false)));
	}

	#[test]
	fn test_parse_number() {
		assert_eq!(from_json("0"), Ok(Number(0.0)));
		assert_eq!(from_json("123"), Ok(Number(123.0)));
		assert_eq!(from_json("-0"), Ok(Number(0.0)));
		assert_eq!(from_json("-123"), Ok(Number(-123.0)));
		assert_eq!(from_json("123.456"), Ok(Number(123.456)));
		assert_eq!(from_json("-123.456"), Ok(Number(-123.456)));
		assert_eq!(from_json("123e1"), Ok(Number(123e1)));
		assert_eq!(from_json("123.456e-10"), Ok(Number(123.456e-10)));
		assert_eq!(from_json("-123E+1"), Ok(Number(-123e1)));
		assert_eq!(from_json("-123.456E-10"), Ok(Number(-123.456e-10)));
	}

	#[test]
	fn test_parse_string() {
		assert_eq!(from_json("\"\""), Ok(String("".into())));
		assert_eq!(from_json("\"abc\""), Ok(String("abc".into())));
		assert_eq!(
			from_json("\"abc\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0001\\u2014\u{2014}def\""),
			Ok(String("abc\"\\/\x08\x0C\n\r\t\x01——def".into()))
		);
	}

	#[test]
	fn test_whitespace() {
		assert_eq!(from_json(" \n\r\t123"), Ok(Number(123.0)));
		assert_eq!(from_json("123 \n\r\t"), Ok(Number(123.0)));
		assert_eq!(from_json(" \n\r\t123 \n\r\t"), Ok(Number(123.0)));
		assert_eq!(from_json(" [ ] "), Ok(Array(vec![])));
		assert_eq!(
			from_json(" [ \"abc\" , 123 ] "),
			Ok(Array(vec![String("abc".into()), Number(123.0)]))
		);
		assert_eq!(
			from_json(" { \"abc\" : 123 , \"\" : null } "),
			Ok(Object(map!{"abc" => Number(123.0), "" => Null}))
		);
	}

	#[test]
	fn test_array() {
		assert_eq!(from_json("[]"), Ok(Array(vec![])));
		assert_eq!(from_json("[1]"), Ok(Array(vec![Number(1.0)])));
		assert_eq!(from_json("[1,[true,\"3\"],4]"), Ok(Array(vec![
			Number(1.0),
			Array(vec![Boolean(true), String("3".into())]),
			Number(4.0),
		])));
	}

	#[test]
	fn test_object() {
		assert_eq!(from_json("{}"), Ok(Object(map!{})));
		assert_eq!(from_json("{\"abc\":true}"), Ok(Object(map!{"abc" => Boolean(true)})));
		assert_eq!(
			from_json("{\"a\":1,\"b\":[\"c\",null,{\"2\":3}],\"d\\ne\":{\"\":{},\"fgh\": \"\"}}"),
			Ok(Object(map!{
				"a" => Number(1.0),
				"b" => Array(vec![String("c".into()), Null, Object(map!{"2" => Number(3.0)})]),
				"d\ne" => Object(map!{"" => Object(map!{}), "fgh" => String("".into())})
			}))
		);
	}

	#[test]
	fn test_errors() {
		assert_eq!(from_json(""), Err("Unexpected end of JSON"));
		assert_eq!(from_json("xyz"), Err("Expected JSON value"));
		assert_eq!(from_json("nil"), Err("Expected JSON value"));
		assert_eq!(from_json("falsy"), Err("Expected JSON value"));
		assert_eq!(from_json("trie"), Err("Expected JSON value"));
		assert_eq!(from_json("-"), Err("Invalid number"));
		assert_eq!(from_json("\"abc"), Err("Expected end of string"));
		assert_eq!(from_json("["), Err("Unexpected end of JSON"));
		assert_eq!(from_json("[a"), Err("Expected JSON value"));
		assert_eq!(from_json("[,"), Err("Expected ']' or value"));
		assert_eq!(from_json("[123"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("[123,"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("[123,,"), Err("Expected value"));
		assert_eq!(from_json("[123,]"), Err("Expected value"));
		assert_eq!(from_json("{"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("{z"), Err("Expected '\"' or '}'"));
		assert_eq!(from_json("{,"), Err("Expected '\"' or '}'"));
		assert_eq!(from_json("{\""), Err("Expected end of string"));
		assert_eq!(from_json("{\"abc\""), Err("Unexpected end of JSON"));
		assert_eq!(from_json("{\"abc\" 2"), Err("Expected ':'"));
		assert_eq!(from_json("{\"abc\":"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("{\"abc\":2"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("{\"abc\":2,"), Err("Unexpected end of JSON"));
		assert_eq!(from_json("{\"abc\":2,,"), Err("Expected '\"'"));
		assert_eq!(from_json("{\"abc\":2,}"), Err("Expected '\"'"));
		// TODO: should duplicate keys error?
		assert_eq!(from_json("{\"a\":1,\"a\":2}"), Ok(Object(map!{"a" => Number(2.0)})));
	}

	const SAMPLE_JSON: &str = r#"
{
    "name": "typescript",
    "author": "Microsoft Corp.",
    "homepage": "https://www.typescriptlang.org/",
    "version": "3.7.0",
    "license": "Apache-2.0",
    "description": "TypeScript is a language for application scale JavaScript development",
    "keywords": [
        "TypeScript",
        "Microsoft",
        "compiler",
        "language",
        "javascript"
    ],
    "bugs": {
        "url": "https://github.com/Microsoft/TypeScript/issues"
    },
    "repository": {
        "type": "git",
        "url": "https://github.com/Microsoft/TypeScript.git"
    },
    "main": "./lib/typescript.js",
    "typings": "./lib/typescript.d.ts",
    "bin": {
        "tsc": "./bin/tsc",
        "tsserver": "./bin/tsserver"
    },
    "engines": {
        "node": ">=4.2.0"
    },
    "devDependencies": {
        "@octokit/rest": "latest",
        "@types/browserify": "latest",
        "@types/chai": "latest",
        "@types/convert-source-map": "latest",
        "@types/del": "latest",
        "@types/glob": "latest",
        "@types/gulp": "^4.0.5",
        "@types/gulp-concat": "latest",
        "@types/gulp-newer": "latest",
        "@types/gulp-rename": "0.0.33",
        "@types/gulp-sourcemaps": "0.0.32",
        "@types/jake": "latest",
        "@types/merge2": "latest",
        "@types/microsoft__typescript-etw": "latest",
        "@types/minimatch": "latest",
        "@types/minimist": "latest",
        "@types/mkdirp": "latest",
        "@types/mocha": "latest",
        "@types/ms": "latest",
        "@types/node": "latest",
        "@types/node-fetch": "^2.3.4",
        "@types/q": "latest",
        "@types/source-map-support": "latest",
        "@types/through2": "latest",
        "@types/travis-fold": "latest",
        "@types/xml2js": "^0.4.0",
        "azure-devops-node-api": "^8.0.0",
        "browser-resolve": "^1.11.2",
        "browserify": "latest",
        "chai": "latest",
        "chalk": "latest",
        "convert-source-map": "latest",
        "del": "latest",
        "fancy-log": "latest",
        "fs-extra": "^6.0.1",
        "gulp": "^4.0.0",
        "gulp-concat": "latest",
        "gulp-insert": "latest",
        "gulp-newer": "latest",
        "gulp-rename": "latest",
        "gulp-sourcemaps": "latest",
        "istanbul": "latest",
        "merge2": "latest",
        "minimist": "latest",
        "mkdirp": "latest",
        "mocha": "latest",
        "mocha-fivemat-progress-reporter": "latest",
        "ms": "latest",
        "node-fetch": "^2.6.0",
        "plugin-error": "latest",
        "pretty-hrtime": "^1.0.3",
        "prex": "^0.4.3",
        "q": "latest",
        "remove-internal": "^2.9.2",
        "simple-git": "^1.113.0",
        "source-map-support": "latest",
        "through2": "latest",
        "travis-fold": "latest",
        "tslint": "latest",
        "typescript": "next",
        "vinyl": "latest",
        "vinyl-sourcemaps-apply": "latest",
        "xml2js": "^0.4.19"
    },
    "scripts": {
        "pretest": "gulp tests",
        "test": "gulp runtests-parallel --light=false",
        "build": "npm run build:compiler && npm run build:tests",
        "build:compiler": "gulp local",
        "build:tests": "gulp tests",
        "start": "node lib/tsc",
        "clean": "gulp clean",
        "gulp": "gulp",
        "jake": "gulp",
        "lint": "gulp lint",
        "setup-hooks": "node scripts/link-hooks.js",
        "update-costly-tests": "node scripts/costly-tests.js"
    },
    "browser": {
        "fs": false,
        "os": false,
        "path": false,
        "@microsoft/typescript-etw": false
    },
    "dependencies": {}
}"#;

	#[test]
	fn test_sample() {
		assert_eq!(from_json(SAMPLE_JSON), Ok(Object(map!{
			"name" => String("typescript".into()),
			"author" => String("Microsoft Corp.".into()),
			"homepage" => String("https://www.typescriptlang.org/".into()),
			"version" => String("3.7.0".into()),
			"license" => String("Apache-2.0".into()),
			"description" => String("TypeScript is a language for application scale JavaScript development".into()),
			"keywords" => Array(vec![
				String("TypeScript".into()),
				String("Microsoft".into()),
				String("compiler".into()),
				String("language".into()),
				String("javascript".into()),
			]),
			"bugs" => Object(map!{
				"url" => String("https://github.com/Microsoft/TypeScript/issues".into())
			}),
			"repository" => Object(map!{
				"type" => String("git".into()),
				"url" => String("https://github.com/Microsoft/TypeScript.git".into())
			}),
			"main" => String("./lib/typescript.js".into()),
			"typings" => String("./lib/typescript.d.ts".into()),
			"bin" => Object(map!{
				"tsc" => String("./bin/tsc".into()),
				"tsserver" => String("./bin/tsserver".into())
			}),
			"engines" => Object(map!{"node" => String(">=4.2.0".into())}),
			"dependencies" => Object(map!{}),
			"devDependencies" => Object(map!{
				"@octokit/rest" => String("latest".into()),
				"@types/browserify" => String("latest".into()),
				"@types/chai" => String("latest".into()),
				"@types/convert-source-map" => String("latest".into()),
				"@types/del" => String("latest".into()),
				"@types/glob" => String("latest".into()),
				"@types/gulp" => String("^4.0.5".into()),
				"@types/gulp-concat" => String("latest".into()),
				"@types/gulp-newer" => String("latest".into()),
				"@types/gulp-rename" => String("0.0.33".into()),
				"@types/gulp-sourcemaps" => String("0.0.32".into()),
				"@types/jake" => String("latest".into()),
				"@types/merge2" => String("latest".into()),
				"@types/microsoft__typescript-etw" => String("latest".into()),
				"@types/minimatch" => String("latest".into()),
				"@types/minimist" => String("latest".into()),
				"@types/mkdirp" => String("latest".into()),
				"@types/mocha" => String("latest".into()),
				"@types/ms" => String("latest".into()),
				"@types/node" => String("latest".into()),
				"@types/node-fetch" => String("^2.3.4".into()),
				"@types/q" => String("latest".into()),
				"@types/source-map-support" => String("latest".into()),
				"@types/through2" => String("latest".into()),
				"@types/travis-fold" => String("latest".into()),
				"@types/xml2js" => String("^0.4.0".into()),
				"azure-devops-node-api" => String("^8.0.0".into()),
				"browser-resolve" => String("^1.11.2".into()),
				"browserify" => String("latest".into()),
				"chai" => String("latest".into()),
				"chalk" => String("latest".into()),
				"convert-source-map" => String("latest".into()),
				"del" => String("latest".into()),
				"fancy-log" => String("latest".into()),
				"fs-extra" => String("^6.0.1".into()),
				"gulp" => String("^4.0.0".into()),
				"gulp-concat" => String("latest".into()),
				"gulp-insert" => String("latest".into()),
				"gulp-newer" => String("latest".into()),
				"gulp-rename" => String("latest".into()),
				"gulp-sourcemaps" => String("latest".into()),
				"istanbul" => String("latest".into()),
				"merge2" => String("latest".into()),
				"minimist" => String("latest".into()),
				"mkdirp" => String("latest".into()),
				"mocha" => String("latest".into()),
				"mocha-fivemat-progress-reporter" => String("latest".into()),
				"ms" => String("latest".into()),
				"node-fetch" => String("^2.6.0".into()),
				"plugin-error" => String("latest".into()),
				"pretty-hrtime" => String("^1.0.3".into()),
				"prex" => String("^0.4.3".into()),
				"q" => String("latest".into()),
				"remove-internal" => String("^2.9.2".into()),
				"simple-git" => String("^1.113.0".into()),
				"source-map-support" => String("latest".into()),
				"through2" => String("latest".into()),
				"travis-fold" => String("latest".into()),
				"tslint" => String("latest".into()),
				"typescript" => String("next".into()),
				"vinyl" => String("latest".into()),
				"vinyl-sourcemaps-apply" => String("latest".into()),
				"xml2js" => String("^0.4.19".into())
			}),
			"scripts" => Object(map!{
				"pretest" => String("gulp tests".into()),
				"test" => String("gulp runtests-parallel --light=false".into()),
				"build" => String("npm run build:compiler && npm run build:tests".into()),
				"build:compiler" => String("gulp local".into()),
				"build:tests" => String("gulp tests".into()),
				"start" => String("node lib/tsc".into()),
				"clean" => String("gulp clean".into()),
				"gulp" => String("gulp".into()),
				"jake" => String("gulp".into()),
				"lint" => String("gulp lint".into()),
				"setup-hooks" => String("node scripts/link-hooks.js".into()),
				"update-costly-tests" => String("node scripts/costly-tests.js".into())
			}),
			"browser" => Object(map!{
				"fs" => Boolean(false),
				"os" => Boolean(false),
				"path" => Boolean(false),
				"@microsoft/typescript-etw" => Boolean(false)
			})
		})));
		assert_eq!(from_json_recursive(SAMPLE_JSON), from_json(SAMPLE_JSON));
	}
}