extern crate nfa;

use nfa::*;

#[test]
fn empty() {
	let re = Regex::StrLiteral(String::new());
	let fa = re.make_fa();
	assert!(fa.accepts(""));
	for c in &['a', ' ', 'é'] {
		let mut s = String::new();
		for _ in 0..100 {
			s.push(*c);
			assert!(!fa.accepts(&s));
		}
	}
}

#[test]
fn nonempty() {
	let re = Regex::StrLiteral("abc".to_string());
	let fa = re.make_fa();
	assert!(!fa.accepts(""));
	assert!(!fa.accepts("a"));
	assert!(!fa.accepts("ab"));
	assert!(fa.accepts("abc"));
	assert!(!fa.accepts("bbc"));
	assert!(!fa.accepts("adc"));
	assert!(!fa.accepts("abb"));
}