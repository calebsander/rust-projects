use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::{FromIterator, IntoIterator};
use std::ops::Add;
use bit_vector::BitVector;
use priority_queue::{MaxHeap, PriorityQueue};

pub struct HuffmanEncoding<T>(HashMap<T, Vec<bool>>);

enum EncodingTree<T> {
	Leaf(T),
	Inner(Box<EncodingTree<T>>, Box<EncodingTree<T>>),
}
struct UnrootedEncodingTree<T, F> {
	tree: EncodingTree<T>,
	frequency: F,
}
impl<T, F: PartialOrd> PartialOrd for UnrootedEncodingTree<T, F> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.frequency.partial_cmp(&self.frequency)
	}
}
impl<T, F: PartialEq> PartialEq for UnrootedEncodingTree<T, F> {
	fn eq(&self, other: &Self) -> bool {
		self.frequency == other.frequency
	}
}

impl<T: Hash + Eq, F: PartialOrd + Add<Output=F>> From<HashMap<T, F>> for HuffmanEncoding<T> {
	fn from(frequencies: HashMap<T, F>) -> Self {
		use EncodingTree::*;

		let mut result = HuffmanEncoding(HashMap::new());
		if frequencies.is_empty() { return result }

		let mut by_frequency = MaxHeap::new();
		for (c, frequency) in frequencies {
			by_frequency.push(UnrootedEncodingTree { tree: Leaf(c), frequency })
		}
		let root = loop {
			let left = by_frequency.next().unwrap();
			match by_frequency.next() {
				Some(right) =>
					by_frequency.push(UnrootedEncodingTree {
						tree: Inner(Box::new(left.tree), Box::new(right.tree)),
						frequency: left.frequency + right.frequency,
					}),
				None => break left.tree,
			}
		};
		result.add_tree(Vec::new(), root);
		result
	}
}
impl<'a, T: 'a + Hash + Eq + Clone> FromIterator<&'a T> for HuffmanEncoding<T> {
	fn from_iter<C: IntoIterator<Item=&'a T>>(corpus: C) -> Self {
		let mut counts = HashMap::new();
		for c in corpus { *counts.entry((*c).clone()).or_insert(0usize) += 1 }
		Self::from(counts)
	}
}
impl<'a, T: 'a + Hash + Eq> HuffmanEncoding<T> {
	pub fn encode<V: IntoIterator<Item=&'a T>>(&self, values: V) -> BitVector {
		let mut bits = BitVector::new();
		for c in values { bits.extend(&self.0[c]) }
		bits
	}
	fn add_tree(&mut self, prefix: Vec<bool>, tree: EncodingTree<T>) {
		use EncodingTree::*;
		match tree {
			Leaf(c) => { self.0.insert(c, prefix); },
			Inner(left, right) => {
				let (mut prefix_zero, mut prefix_one) = (prefix.clone(), prefix.clone());
				prefix_zero.push(false);
				prefix_one.push(true);
				self.add_tree(prefix_zero, *left);
				self.add_tree(prefix_one, *right);
			}
		}
	}
}