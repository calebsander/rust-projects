pub struct PermuteIter<T> {
	elems: Vec<T>,
	items_left: usize,
	indices: Option<Vec<usize>>,
}

impl<T: Clone> PermuteIter<T> {
	pub fn new(elems: Vec<T>) -> Self {
		let items_left = Self::permutation_count(&elems);
		PermuteIter { elems, items_left, indices: None }
	}

	fn permutation_count(elems: &Vec<T>) -> usize {
		(2..=elems.len()).product()
	}
}

impl<T: Clone> Iterator for PermuteIter<T> {
	type Item = Vec<T>;

	fn next(&mut self) -> Option<Vec<T>> {
		match &mut self.indices {
			Some(indices) => {
				for (i, choice) in indices.iter_mut().enumerate().rev() {
					let index = i + *choice;
					self.elems.swap(i, index);
					let next_index = index + 1;
					if next_index < self.elems.len() {
						*choice += 1;
						self.elems.swap(next_index, i);
						self.items_left -= 1;
						return Some(self.elems.clone());
					}
					else { *choice = 0 }
				}
				None
			},
			None => {
				self.indices = Some(vec![0; self.elems.len()]);
				Some(self.elems.clone())
			}
		}
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.items_left, Some(self.items_left))
	}
	fn count(self) -> usize {
		self.items_left
	}
}
impl<T: Clone> ExactSizeIterator for PermuteIter<T> {}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashSet;

	fn factorial(n: usize) -> usize {
		if n < 2 { 1 } else { n * factorial(n - 1) }
	}

	#[test]
	fn test_zero() {
		let iter = PermuteIter::<i32>::new(vec![]);
		assert_eq!(iter.len(), 1);
		assert_eq!(iter.collect::<Vec<_>>(), vec![vec![]]);
	}
	#[test]
	fn test_one() {
		let iter = PermuteIter::new(vec![1]);
		assert_eq!(iter.len(), 1);
		assert_eq!(iter.collect::<Vec<_>>(), vec![vec![1]]);
	}
	#[test]
	fn test_small() {
		let iter = PermuteIter::new(vec![1, 2, 3]);
		assert_eq!(iter.len(), 6);
		assert_eq!(iter.collect::<Vec<_>>(), vec![
			vec![1, 2, 3],
			vec![1, 3, 2],
			vec![2, 1, 3],
			vec![2, 3, 1],
			vec![3, 2, 1],
			vec![3, 1, 2],
		]);
	}
	#[test]
	fn test_large() {
		let items: Vec<_> = (0..10).collect();
		let iter = PermuteIter::new(items.clone());
		assert_eq!(iter.len(), factorial(items.len()));
		let permutations: HashSet<_> = iter.collect();
		assert_eq!(permutations.len(), factorial(items.len()));
		for mut permutation in permutations {
			permutation.sort_unstable();
			assert_eq!(permutation, items);
		}
	}
}