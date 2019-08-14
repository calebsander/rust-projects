extern crate nfa;

use nfa::*;

#[test]
fn test() {
	let re = Regex::Dot;
	let fa = re.make_fa();
	for c in &['a', ' ', 'é'] {
		let mut s = String::new();
		for len in 0..100 {
			assert!(fa.accepts(&s) == (len == 1));
			s.push(*c);
		}
	}
}