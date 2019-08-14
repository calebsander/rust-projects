extern crate nfa;

use nfa::*;

#[test]
fn test() {
	let re = Regex::Empty;
	let fa = re.make_fa();
	assert!(fa.accepts(""));
	for &c in &['a', ' ', 'é'] {
		let mut s = String::new();
		for _ in 1..100 {
			s.push(c);
			assert!(!fa.accepts(&s));
		}
	}
}