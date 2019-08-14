struct DiagonalResult {
	insertion: bool,
	start_b_index: usize,
	end_b_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffElement<'a, T> {
	Same(usize),
	Change(usize, &'a [T]),
}

fn make_diff<'b, T: PartialEq>(
	b: &'b [T],
	mut frontiers: Vec<Vec<DiagonalResult>>
) -> Vec<DiffElement<'b, T>> {
	use DiffElement::*;

	let mut diff = vec![];
	let mut diagonal = frontiers.last().unwrap().len() - 1;
	loop {
		// Combine consecutive insertions and deletions into a single Change,
		// since they are interchangeable.
		// We take 1 Same block (and its optional insertion/deletion) and then
		// keep taking insertions/deletions until we hit a Same block.
		let (mut deletions, mut insertions) = (0, 0);
		let mut end_result = None;
		loop {
			let DiagonalResult { insertion, start_b_index, end_b_index } =
				frontiers.last().unwrap()[diagonal];
			match end_result {
				Some(_) => if start_b_index < end_b_index { break }
				None => end_result = Some((start_b_index, end_b_index - start_b_index)),
			}
			frontiers.pop();
			// The first frontier cannot have an insertion or deletion
			if frontiers.is_empty() { break }

			if insertion {
				insertions += 1;
				diagonal -= 1;
			}
			else { deletions += 1 }
		};
		let (end_b_index, same_count) = end_result.unwrap();
		// same_count can only be 0 at the end of the diff
		if same_count > 0 { diff.push(Same(same_count)) }
		// deletions and insertions can both be 0 only at the start of the diff
		if deletions + insertions > 0 {
			diff.push(Change(deletions, &b[(end_b_index - insertions)..end_b_index]))
		}
		if frontiers.is_empty() { break }
	}
	diff.reverse();
	return diff;
}

pub fn diff<'b, T: PartialEq>(a: &[T], b: &'b [T]) -> Vec<DiffElement<'b, T>> {
	let mut frontiers: Vec<Vec<DiagonalResult>> = vec![];
	let empty = vec![];
	loop {
		let diff_length = frontiers.len();
		let last_frontier = frontiers.last().unwrap_or(&empty);
		let mut frontier = Vec::with_capacity(diff_length + 1);
		let mut insert_b_index = 0;
		for diagonal in 0..=diff_length {
			let delete_b_index = match last_frontier.get(diagonal) {
				Some(result) => result.end_b_index,
				None => 0
			};
			let insertion = insert_b_index > delete_b_index;
			let start_b_index =
				if insertion { insert_b_index } else { delete_b_index };
			let mut end_a_index = start_b_index + diff_length - (diagonal << 1);
			let mut end_b_index = start_b_index;
			let done = loop {
				match (a.get(end_a_index), b.get(end_b_index)) {
					(Some(a_elem), Some(b_elem)) => {
						if a_elem == b_elem {
							end_a_index += 1;
							end_b_index += 1;
						}
						else { break false }
					},
					(Some(_), None) | (None, Some(_)) => break false,
					(None, None) => break true,
				}
			};
			frontier.push(DiagonalResult { insertion, start_b_index, end_b_index });
			if done {
				frontiers.push(frontier);
				return make_diff(b, frontiers);
			}

			insert_b_index = delete_b_index + 1;
		}
		frontiers.push(frontier);
	}
}

pub fn diff_len<T>(diff: &Vec<DiffElement<'_, T>>) -> usize {
	use DiffElement::*;

	diff.into_iter()
		.map(|element| match element {
			Same(_) => 0,
			Change(deletions, insertions) => deletions + insertions.len(),
		})
		.sum()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::slice;
	use DiffElement::*;

	fn join_slices<'a, T>(first: &'a [T], second: &'a [T]) -> &'a [T] {
		let (first_start, first_len) = (first.as_ptr(), first.len());
		unsafe {
			if first_start.add(first_len) != second.as_ptr() {
				panic!("Slices are not adjacent");
			}
			slice::from_raw_parts(first_start, first_len + second.len())
		}
	}

	fn diff_brute<'b, T: PartialEq>(a: &[T], b: &'b [T]) -> Vec<DiffElement<'b, T>> {
		if a.is_empty() {
			if b.is_empty() { vec![] } else { vec![Change(0, b)] }
		}
		else if b.is_empty() { vec![Change(a.len(), &[])] }
		else {
			if a[0] == b[0] {
				let mut diff_rest = diff_brute(&a[1..], &b[1..]);
				match diff_rest.get_mut(0) {
					Some(Same(ref mut count)) => *count += 1,
					_ => diff_rest.insert(0, Same(1)),
				}
				diff_rest
			}
			else {
				let mut diff_rest_left = diff_brute(&a[1..], b);
				let mut diff_rest_right = diff_brute(a, &b[1..]);
				if diff_len(&diff_rest_left) < diff_len(&diff_rest_right) {
					match diff_rest_left.get_mut(0) {
						Some(Change(ref mut count, _)) => *count += 1,
						_ => diff_rest_left.insert(0, Change(1, &[])),
					}
					diff_rest_left
				}
				else {
					let inserted = &b[..1];
					match diff_rest_right.get_mut(0) {
						Some(Change(_, ref mut slice)) => *slice = join_slices(inserted, *slice),
						_ => diff_rest_right.insert(0, Change(0, inserted)),
					}
					diff_rest_right
				}
			}
		}
	}

	#[test]
	fn test_same() {
		let mut items = vec![];
		assert_eq!(diff(&items, &items), vec![]);
		assert_eq!(diff_brute(&items, &items), vec![]);
		for i in 1..100 {
			items.push(i);
			assert_eq!(diff(&items, &items), vec![Same(i)]);
			assert_eq!(diff_brute(&items, &items), vec![Same(i)]);
		}
	}

	#[test]
	fn test_insertions() {
		let initial = vec![1, 2, 3, 4, 5];
		let inserts = [6, 7];
		for insert1 in 0..initial.len() {
			for insert2 in insert1..initial.len() {
				let mut inserted = initial.clone();
				inserted.insert(insert1, inserts[0]);
				inserted.insert(insert2 + 1, inserts[1]);
				let mut target_diff = vec![];
				if insert1 > 0 { target_diff.push(Same(insert1)) }
				if insert1 < insert2 {
					target_diff.extend(&[
						Change(0, &inserts[..1]),
						Same(insert2 - insert1),
						Change(0, &inserts[1..]),
					])
				}
				else { target_diff.push(Change(0, &inserts)) }
				if insert2 < initial.len() {
					target_diff.push(Same(initial.len() - insert2))
				}
				let diff_result = diff(&initial, &inserted);
				assert_eq!(diff_result, target_diff);
				assert_eq!(diff_result, diff_brute(&initial, &inserted));
			}
		}
	}

	#[test]
	fn test_deletions() {
		let initial = vec![1, 2, 3, 4, 5, 6, 7];
		for delete1 in 0..(initial.len() - 1) {
			for delete2 in delete1..(initial.len() - 1) {
				let mut deleted = initial.clone();
				deleted.remove(delete1);
				deleted.remove(delete2);
				let mut target_diff = vec![];
				if delete1 > 0 { target_diff.push(Same(delete1)) }
				if delete1 < delete2 {
					target_diff.extend(&[
						Change(1, &[]),
						Same(delete2 - delete1),
						Change(1, &[]),
					])
				}
				else { target_diff.push(Change(2, &[])) }
				if delete2 < initial.len() - 2 {
					target_diff.push(Same(initial.len() - 2 - delete2))
				}
				let diff_result = diff(&initial, &deleted);
				assert_eq!(diff_result, target_diff);
				assert_eq!(diff_result, diff_brute(&initial, &deleted));
			}
		}
	}
}