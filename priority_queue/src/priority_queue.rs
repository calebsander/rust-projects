use std::fmt::{Debug, Formatter, Result};
use std::iter::FromIterator;

pub trait PriorityQueue<T> : Iterator<Item=T> {
	fn new() -> Self;

	fn is_empty(&self) -> bool;
	fn len(&self) -> usize;
	fn push(&mut self, value: T);
	fn peek(&self) -> Option<&T>;
}

#[derive(Clone, Default)]
pub struct MaxHeap<T>(Vec<T>);

const ROOT_INDEX: usize = 0;
fn get_left_child(index: usize) -> usize {
	(index << 1) + 1
}
fn get_right_sibling(index: usize) -> usize {
	index + 1
}
fn get_parent(index: usize) -> usize {
	(index - 1) >> 1
}

impl<T> MaxHeap<T> {
	fn swap_to(&mut self, current_index: &mut usize, new_index: usize) {
		self.0.swap(*current_index, new_index);
		*current_index = new_index;
	}
}

impl<T: PartialOrd> Iterator for MaxHeap<T> {
	type Item = T;

	fn next(&mut self) -> Option<T> {
		if self.is_empty() { return None }

		let mut current_index = ROOT_INDEX;
		let result = self.0.swap_remove(current_index);
		loop {
			let left_child_index = get_left_child(current_index);
			match self.0.get(left_child_index) {
				None => break, // propagated to a leaf, so we're done
				Some(left_child) => {
					let right_child_index = get_right_sibling(left_child_index);
					let (max_child_index, max_child) =
						match self.0.get(right_child_index) {
							Some(right_child) if right_child > left_child =>
								(right_child_index, right_child),
							_ => (left_child_index, left_child),
						};
					if self.0[current_index] >= *max_child { break }

					self.swap_to(&mut current_index, max_child_index);
				},
			};
		}
		Some(result)
	}
}
impl<T: PartialOrd> PriorityQueue<T> for MaxHeap<T> {
	fn new() -> Self {
		MaxHeap(vec![])
	}

	fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
	fn len(&self) -> usize {
		self.0.len()
	}
	fn push(&mut self, value: T) {
		let mut current_index = self.0.len();
		self.0.push(value);
		while current_index != ROOT_INDEX {
			let parent_index = get_parent(current_index);
			if self.0[parent_index] >= self.0[current_index] { break }

			self.swap_to(&mut current_index, parent_index)
		}
	}
	fn peek(&self) -> Option<&T> {
		self.0.first()
	}
}

impl<T: Clone + PartialOrd> PartialEq for MaxHeap<T> {
	fn eq(&self, other: &Self) -> bool {
		self.len() == other.len() &&
			self.clone().zip(other.clone()).all(|(x, y)| x == y)
	}
}
impl<T: Clone + PartialOrd + Eq> Eq for MaxHeap<T> {}
impl<T: PartialOrd> FromIterator<T> for MaxHeap<T> {
	fn from_iter<I: IntoIterator<Item=T>>(items: I) -> Self {
		let iter = items.into_iter();
		let (lower, upper) = iter.size_hint();
		let mut result = MaxHeap(Vec::with_capacity(upper.unwrap_or(lower)));
		for item in iter { result.push(item) }
		result
	}
}
impl<T: Debug> MaxHeap<T> {
	fn to_lines(&self, index: usize, max_depth: usize) -> Vec<String> {
		if max_depth == 0 { return vec!["...".to_string()] }

		let left_child_index = get_left_child(index);
		if left_child_index >= self.0.len() {
			return vec![format!("{:?}", self.0[index])]
		}

		let mut lines = vec![String::new(), String::new()];
		lines.append(&mut self.to_lines(left_child_index, max_depth - 1));
		lines[1] = format!("{:^1$}", '/', lines[2].len());
		let right_child_index = get_right_sibling(left_child_index);
		if right_child_index < self.0.len() {
			let right_lines = self.to_lines(right_child_index, max_depth - 1);
			let right_width = right_lines[0].len();
			lines[1] += &format!(" {:^1$}", '\\', right_width);
			for (line, right_line) in lines.iter_mut().skip(2).zip(right_lines.iter()) {
				*line += " ";
				*line += &right_line;
			}
		}
		lines[0] = format!("{:^1$?}", self.0[index], lines[1].len());
		lines
	}
}
impl<T: Debug> Debug for MaxHeap<T> {
	fn fmt(&self, f: &mut Formatter) -> Result {
		if self.0.is_empty() { return f.write_str("(empty)") }

		for (i, line) in self.to_lines(ROOT_INDEX, 5).iter().enumerate() {
			if i > 0 { f.write_str("\n")? }
			f.write_str(line)?
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::cmp::{Ordering, Reverse};
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};

	fn sort(list: &[i32]) -> Vec<i32> {
		let mut priority_queue = MaxHeap::new();
		for elem in list {
			priority_queue.push(Reverse(*elem))
		}
		let mut sorted = vec![];
		for elem in priority_queue {
			sorted.push(elem.0)
		}
		sorted
	}

	#[derive(Debug)]
	struct KeyedValue(&'static str);
	impl KeyedValue {
		fn get_key(&self) -> usize {
			self.0.len()
		}
	}
	impl PartialEq for KeyedValue {
		fn eq(&self, other: &Self) -> bool {
			self.get_key() == other.get_key()
		}
	}
	impl Eq for KeyedValue {}
	impl Ord for KeyedValue {
		fn cmp(&self, other: &Self) -> Ordering {
			self.get_key().cmp(&other.get_key())
		}
	}
	impl PartialOrd for KeyedValue {
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			Some(self.cmp(other))
		}
	}

	#[test]
	fn test_empty() {
		assert_eq!(sort(&[]), vec![]);
	}

	#[test]
	fn test_small() {
		assert_eq!(sort(&[1, 2, 3]), vec![1, 2, 3]);
		assert_eq!(sort(&[1, 3, 2]), vec![1, 2, 3]);
		assert_eq!(sort(&[2, 1, 3]), vec![1, 2, 3]);
		assert_eq!(sort(&[2, 3, 1]), vec![1, 2, 3]);
		assert_eq!(sort(&[3, 1, 2]), vec![1, 2, 3]);
		assert_eq!(sort(&[3, 2, 1]), vec![1, 2, 3]);
	}

	#[test]
	fn test_large() {
		let mut priority_queue = MaxHeap::new();
		assert_eq!(priority_queue.len(), 0);
		assert!(priority_queue.is_empty());
		let mut inserted_values = vec![];
		let mut insert_value = 12345u16;
		let mut hasher = DefaultHasher::new();
		for _ in 0..1000000 {
			priority_queue.push(insert_value);
			inserted_values.push(insert_value);
			insert_value.hash(&mut hasher);
			insert_value = hasher.finish() as u16;
		}
		assert_eq!(priority_queue.len(), 1000000);
		assert!(!priority_queue.is_empty());
		inserted_values.sort_unstable();
		while priority_queue.len() > 500000 {
			assert_eq!(priority_queue.next(), inserted_values.pop())
		}
		for _ in 0..500000 {
			priority_queue.push(insert_value);
			inserted_values.push(insert_value);
			insert_value.hash(&mut hasher);
			insert_value = hasher.finish() as u16;
		}
		assert_eq!(priority_queue.len(), 1000000);
		assert!(!priority_queue.is_empty());
		inserted_values.sort_unstable();
		for value in inserted_values.into_iter().rev() {
			assert_eq!(priority_queue.next(), Some(value))
		}
		assert_eq!(priority_queue.len(), 0);
		assert!(priority_queue.is_empty());
		assert_eq!(priority_queue.next(), None);
	}

	#[test]
	fn test_duplicates() {
		let mut priority_queue = MaxHeap::new();
		for _ in 0..10 {
			priority_queue.push(1);
			priority_queue.push(2);
			priority_queue.push(3);
		}
		for _ in 0..10 {
			assert_eq!(priority_queue.next(), Some(3))
		}
		for _ in 0..10 {
			assert_eq!(priority_queue.next(), Some(2))
		}
		for _ in 0..10 {
			assert_eq!(priority_queue.next(), Some(1))
		}
		for _ in 0..10 {
			assert_eq!(priority_queue.next(), None)
		}
	}

	#[test]
	fn test_floats() {
		let mut priority_queue = MaxHeap::new();
		for i in 0..100 {
			priority_queue.push((i % 10 * 10 + i / 10) as f64)
		}
		for i in (0..100).rev() {
			assert_eq!(priority_queue.next(), Some(i as f64))
		}
		assert_eq!(priority_queue.next(), None)
	}

	#[test]
	fn test_keyed() {
		let mut priority_queue = MaxHeap::new();
		priority_queue.push(KeyedValue("zero"));
		priority_queue.push(KeyedValue("one"));
		priority_queue.push(KeyedValue("2"));
		priority_queue.push(KeyedValue("three"));
		priority_queue.push(KeyedValue("eleven"));
		assert_eq!(priority_queue.next(), Some(KeyedValue("eleven")));
		assert_eq!(priority_queue.next(), Some(KeyedValue("three")));
		assert_eq!(priority_queue.next(), Some(KeyedValue("zero")));
		assert_eq!(priority_queue.next(), Some(KeyedValue("one")));
		assert_eq!(priority_queue.next(), Some(KeyedValue("2")));
		assert_eq!(priority_queue.next(), None);
	}

	#[test]
	fn test_debug() {
		let mut priority_queue = MaxHeap::new();
		assert_eq!(format!("{:?}", priority_queue), "(empty)");
		priority_queue.push(1);
		assert_eq!(format!("{:?}", priority_queue), "1");
		priority_queue.push(2);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			"2",
			"/",
			"1",
		]);
		priority_queue.push(3);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			" 3 ",
			"/ \\",
			"1 2",
		]);
		priority_queue.push(4);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			" 4 ",
			"/ \\",
			"3 2",
			"/",
			"1",
		]);
		priority_queue.push(5);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			"  5  ",
			" /  \\",
			" 4  2",
			"/ \\",
			"1 3",
		]);
		priority_queue.push(6);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			"  6  ",
			" /  \\",
			" 4  5",
			"/ \\ /",
			"1 3 2",
		]);
		priority_queue.push(7);
		assert_eq!(format!("{:?}", priority_queue).split('\n').collect::<Vec<_>>(), [
			"   7   ",
			" /   \\ ",
			" 4   6 ",
			"/ \\ / \\",
			"1 3 2 5",
		]);
	}
}