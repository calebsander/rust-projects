extern crate nfa;

use nfa::*;

#[test]
fn test() {
	let re = Regex::CharLiteral('é');
	let fa = re.make_fa();
	let mut s = String::new();
	for len in 0..100 {
		assert!(fa.accepts(&s) == (len == 1));
		s.push('é');
	}
	for c in &['a', ' ', '☃'] {
		s.clear();
		for _ in 0..100 {
			assert!(!fa.accepts(&s));
			s.push(*c);
		}
	}
}