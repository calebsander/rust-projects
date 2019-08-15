use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::hash::Hash;
use std::iter::{FromIterator, IntoIterator};
use std::ops::Add;
use bit_vector::BitVector;

enum EncodingTree<T> {
	Leaf(T),
	Inner(Box<Self>, Box<Self>),
}
struct UnrootedEncodingTree<T, F> {
	tree: EncodingTree<T>,
	frequency: F,
}
impl<T, F: PartialEq> PartialEq for UnrootedEncodingTree<T, F> {
	fn eq(&self, other: &Self) -> bool {
		self.frequency == other.frequency
	}
}
impl<T, F: Eq> Eq for UnrootedEncodingTree<T, F> {}
impl<T, F: PartialOrd> PartialOrd for UnrootedEncodingTree<T, F> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.frequency.partial_cmp(&self.frequency)
	}
}
impl<T, F: Ord> Ord for UnrootedEncodingTree<T, F> {
	fn cmp(&self, other: &Self) -> Ordering {
		other.frequency.cmp(&self.frequency)
	}
}

pub struct HuffmanEncoding<T> {
	encodings: HashMap<T, BitVector>,
	decode_tree: EncodingTree<T>,
}

impl<T: Hash + Eq + Clone> From<EncodingTree<T>> for HuffmanEncoding<T> {
	fn from(decode_tree: EncodingTree<T>) -> Self {
		let mut encodings = HashMap::new();
		add_tree(&mut encodings, BitVector::new(), &decode_tree);
		HuffmanEncoding { encodings, decode_tree }
	}
}
impl<T: Hash + Eq + Clone, F: Ord + Add<Output=F>> From<HashMap<T, F>> for HuffmanEncoding<T> {
	fn from(frequencies: HashMap<T, F>) -> Self {
		use EncodingTree::*;

		let mut by_frequency = BinaryHeap::new();
		for (c, frequency) in frequencies {
			by_frequency.push(UnrootedEncodingTree { tree: Leaf(c), frequency })
		}
		let root = loop {
			let left = by_frequency.pop().expect("Cannot construct an empty huffman tree");
			match by_frequency.pop() {
				Some(right) =>
					by_frequency.push(UnrootedEncodingTree {
						tree: Inner(Box::new(left.tree), Box::new(right.tree)),
						frequency: left.frequency + right.frequency,
					}),
				None => break left.tree,
			}
		};
		Self::from(root)
	}
}
impl<T: Hash + Eq + Clone> FromIterator<T> for HuffmanEncoding<T> {
	fn from_iter<C: IntoIterator<Item=T>>(corpus: C) -> Self {
		let mut counts = HashMap::new();
		for c in corpus { *counts.entry(c).or_insert(0usize) += 1 }
		Self::from(counts)
	}
}
impl<'a, T: 'a + Hash + Eq + Clone> FromIterator<&'a T> for HuffmanEncoding<T> {
	fn from_iter<C: IntoIterator<Item=&'a T>>(corpus: C) -> Self {
		Self::from_iter(corpus.into_iter().cloned())
	}
}
impl<'a, T: 'a + Hash + Eq + Clone> HuffmanEncoding<T> {
	pub fn decode<I: IntoIterator<Item=bool>>(&self, bits: I, count: usize) -> Vec<T> {
		let mut iter = bits.into_iter();
		let mut result = Vec::with_capacity(count);
		for _ in 0..count { result.push(decode_from(&mut iter, &self.decode_tree)) }
		result
	}
	pub fn encode<V: IntoIterator<Item=T>>(&self, values: V) -> BitVector {
		let mut bits = BitVector::new();
		for c in values { bits.extend(&self.encodings[&c]) }
		bits
	}
	pub fn encode_ref<V: IntoIterator<Item=&'a T>>(&self, values: V) -> BitVector {
		let mut bits = BitVector::new();
		for c in values { bits.extend(&self.encodings[c]) }
		bits
	}
}

fn add_tree<T: Hash + Eq + Clone>(
	encodings: &mut HashMap<T, BitVector>, prefix: BitVector, tree: &EncodingTree<T>
) {
	use EncodingTree::*;

	match tree {
		Leaf(c) => {
			let existing_encoding = encodings.insert(c.clone(), prefix);
			assert!(existing_encoding.is_none());
		},
		Inner(left, right) => {
			let (mut prefix_zero, mut prefix_one) = (prefix.clone(), prefix.clone());
			prefix_zero.push(false);
			prefix_one.push(true);
			add_tree(encodings, prefix_zero, left);
			add_tree(encodings, prefix_one, right);
		}
	}
}
fn decode_from<T: Clone>(iter: &mut Iterator<Item=bool>, tree: &EncodingTree<T>) -> T {
	use EncodingTree::*;

	match tree {
		Leaf(c) => c.clone(),
		Inner(left, right) => {
			let child =
				if iter.next().expect("Encoding is not long enough") { right }
				else { left };
			decode_from(iter, child)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_example() {
		// From https://en.wikipedia.org/wiki/Huffman_coding#/media/File:Huffman_coding_visualisation.svg
		// (with one _ removed to remove indeterminism)
		let text = "ADEAD_DAD_CEDED_A_BAD_BABE_A_BEADED_ABACA_BED";
		let huffman_tree = HuffmanEncoding::from_iter(text.chars());
		let encoded = huffman_tree.encode(text.chars());
		let expected = "10011101001000110010011101100111001001000111110010011111011111100010001111110100111001001011111011101000111111001"
			.chars()
			.map(|c| c == '1');
		assert_eq!(encoded, BitVector::from_iter(expected));
		assert_eq!(huffman_tree.decode(encoded, text.len()), text.chars().collect::<Vec<_>>());
	}
}