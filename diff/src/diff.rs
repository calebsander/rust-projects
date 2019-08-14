use std::slice;

struct DiagonalResult {
	insertion: bool,
	start_b_index: usize,
	end_b_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffElement<'a, T> {
	Same(usize),
	Delete(usize),
	Insert(&'a [T]),
}

fn join_slices<'a, T>(first: &'a [T], second: &'a [T]) -> &'a [T] {
	let (first_start, first_len) = (first.as_ptr(), first.len());
	unsafe {
		if first_start.add(first_len) != second.as_ptr() {
			panic!("Slices are not adjacent");
		}
		slice::from_raw_parts(first_start, first_len + second.len())
	}
}

fn make_diff<'b, T: PartialEq>(
	b: &'b [T],
	mut frontiers: Vec<Vec<DiagonalResult>>
) -> Vec<DiffElement<'b, T>> {
	use DiffElement::*;

	let mut diff = vec![];
	let mut diagonal = frontiers.last().unwrap().len() - 1;
	loop {
		let frontier = frontiers.pop().unwrap();
		let choice = &frontier[diagonal];
		let start_b_index = choice.start_b_index;
		let same_count = choice.end_b_index - start_b_index;
		if same_count > 0 { diff.push(Same(same_count)) }
		if frontiers.is_empty() { break }

		if choice.insertion {
			let inserted = slice::from_ref(&b[start_b_index - 1]);
			match diff.last_mut() {
				Some(Insert(ref mut slice)) => *slice = join_slices(inserted, *slice),
				_ => diff.push(Insert(inserted)),
			}
			diagonal -= 1;
		}
		else {
			match diff.last_mut() {
				Some(Delete(ref mut count)) => *count += 1,
				_ => diff.push(Delete(1)),
			}
		}
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
				if insertion { insert_b_index }
				else { delete_b_index };
			let mut a_index = start_b_index + diff_length - (diagonal << 1);
			let mut b_index = start_b_index;
			let done = loop {
				match (a.get(a_index), b.get(b_index)) {
					(Some(a_elem), Some(b_elem)) => {
						if a_elem == b_elem {
							a_index += 1;
							b_index += 1;
						}
						else { break false }
					},
					(Some(_), None) | (None, Some(_)) => break false,
					(None, None) => break true,
				}
			};
			frontier.push(DiagonalResult {
				insertion,
				start_b_index,
				end_b_index: b_index,
			});
			if done {
				frontiers.push(frontier);
				return make_diff(b, frontiers);
			}

			insert_b_index = delete_b_index + 1;
		}
		frontiers.push(frontier);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use DiffElement::*;

	fn diff_brute<'b, T: PartialEq>(a: &[T], b: &'b [T]) -> (usize, Vec<DiffElement<'b, T>>) {
		if a.is_empty() {
			if b.is_empty() { (0, vec![]) } else { (b.len(), vec![Insert(b)]) }
		}
		else if b.is_empty() { (a.len(), vec![Delete(a.len())]) }
		else {
			if a[0] == b[0] {
				let (diff_count, mut diff_rest) = diff_brute(&a[1..], &b[1..]);
				match diff_rest.get_mut(0) {
					Some(Same(ref mut count)) => *count += 1,
					_ => diff_rest.insert(0, Same(1)),
				}
				(diff_count, diff_rest)
			}
			else {
				let (diff_count_left, mut diff_rest_left) = diff_brute(&a[1..], b);
				let (diff_count_right, mut diff_rest_right) = diff_brute(a, &b[1..]);
				if diff_count_left < diff_count_right {
					match diff_rest_left.get_mut(0) {
						Some(Delete(ref mut count)) => *count += 1,
						_ => diff_rest_left.insert(0, Delete(1)),
					}
					(diff_count_left + 1, diff_rest_left)
				}
				else {
					let inserted = &b[..1];
					match diff_rest_right.get_mut(0) {
						Some(Insert(ref mut slice)) => *slice = join_slices(inserted, *slice),
						_ => diff_rest_right.insert(0, Insert(inserted)),
					}
					(diff_count_right + 1, diff_rest_right)
				}
			}
		}
	}

	#[test]
	fn test_same() {
		let mut items = vec![];
		assert_eq!(diff(&items, &items), vec![]);
		for i in 1..100 {
			items.push(i);
			assert_eq!(diff(&items, &items), vec![Same(i)]);
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
						Insert(&inserts[..1]),
						Same(insert2 - insert1),
						Insert(&inserts[1..]),
					])
				}
				else { target_diff.push(Insert(&inserts)) }
				if insert2 < initial.len() {
					target_diff.push(Same(initial.len() - insert2))
				}
				let diff_result = diff(&initial, &inserted);
				assert_eq!(diff_result, target_diff);
				assert_eq!(diff_result, diff_brute(&initial, &inserted).1);
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
					target_diff.extend(&[Delete(1), Same(delete2 - delete1), Delete(1)])
				}
				else { target_diff.push(Delete(2)) }
				if delete2 < initial.len() - 2 {
					target_diff.push(Same(initial.len() - 2 - delete2))
				}
				let diff_result = diff(&initial, &deleted);
				assert_eq!(diff_result, target_diff);
				assert_eq!(diff_result, diff_brute(&initial, &deleted).1);
			}
		}
	}
}