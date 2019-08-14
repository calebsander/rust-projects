use std::iter::{FromIterator, IntoIterator};
use std::mem;

pub struct BitVector {
	len: usize,
	words: Vec<usize>,
}

const WORD_BYTES: usize = mem::size_of::<usize>();
const WORD_BITS: usize = WORD_BYTES * 8;
const LOG_WORD_BITS: u8 = WORD_BITS.trailing_zeros() as u8;

impl BitVector {
	pub fn new() -> Self {
		BitVector { len: 0, words: Vec::new() }
	}
	pub fn with_capacity(capacity: usize) -> Self {
		BitVector { len: 0, words: Vec::with_capacity(Self::to_words_ceil(capacity)) }
	}

	pub fn capacity(&self) -> usize {
		Self::from_word_index(self.words.capacity())
	}
	pub fn clear(&mut self) {
		self.len = 0
	}
	pub fn get(&self, index: usize) -> Option<bool> {
		if index >= self.len { return None }

		let word_index = Self::to_word_index(index);
		let word_offset = Self::to_word_offset(index);
		Some(self.words[word_index] >> word_offset & 1 > 0)
	}
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}
	pub fn len(&self) -> usize {
		self.len
	}
	pub fn pop(&mut self) -> Option<bool> {
		if self.len == 0 { return None }

		self.len -= 1;
		let word_index = Self::to_word_index(self.len);
		let word_offset = Self::to_word_offset(self.len);
		Some(self.words[word_index] >> word_offset & 1 > 0)
	}
	pub fn push(&mut self, value: bool) {
		let word_index = Self::to_word_index(self.len);
		if word_index == self.words.len() {
			self.words.push(value as usize)
		}
		else {
			let set_bit = 1 << Self::to_word_offset(self.len);
			if value { self.words[word_index] |= set_bit }
			else { self.words[word_index] &= !set_bit }
		}
		self.len += 1;
	}
	pub fn set(&mut self, index: usize, value: bool) -> Option<()> {
		if index >= self.len { return None }

		let word_index = Self::to_word_index(index);
		let set_bit = 1 << Self::to_word_offset(self.len);
		if value { self.words[word_index] |= set_bit }
		else { self.words[word_index] &= !set_bit }
		Some(())
	}

	pub fn bytes(&self) -> Bytes {
		Bytes { bits: self, index: 0, current_word: 0 }
	}

	fn to_word_index(bit_index: usize) -> usize {
		bit_index >> LOG_WORD_BITS
	}
	fn to_word_offset(bit_index: usize) -> usize {
		bit_index & (WORD_BITS - 1)
	}
	fn to_words_ceil(bits: usize) -> usize {
		Self::to_word_index(bits) + (Self::to_word_offset(bits) > 0) as usize
	}
	fn from_word_index(word_index: usize) -> usize {
		word_index << LOG_WORD_BITS
	}
}

impl Extend<bool> for BitVector {
	fn extend<I: IntoIterator<Item=bool>>(&mut self, values: I) {
		let iter = values.into_iter();
		let (additional, _) = iter.size_hint();
		self.words.reserve(Self::to_words_ceil(additional));
		for value in iter { self.push(value) }
	}
}
impl<'a> Extend<&'a bool> for BitVector {
	fn extend<I: IntoIterator<Item=&'a bool>>(&mut self, values: I) {
		self.extend(values.into_iter().cloned())
	}
}

impl FromIterator<bool> for BitVector {
	fn from_iter<I: IntoIterator<Item=bool>>(iter: I) -> Self {
		let mut result = BitVector::new();
		result.extend(iter);
		result
	}
}
impl<'a> FromIterator<&'a bool> for BitVector {
	fn from_iter<I: IntoIterator<Item=&'a bool>>(iter: I) -> Self {
		let mut result = BitVector::new();
		result.extend(iter);
		result
	}
}

pub struct IntoIter {
	bits: BitVector,
	index: usize,
}

impl Iterator for IntoIter {
	type Item = bool;

	fn next(&mut self) -> Option<bool> {
		let result = self.bits.get(self.index);
		self.index += 1;
		result
	}
}

pub struct Iter<'a> {
	bits: &'a BitVector,
	index: usize,
}

impl<'a> Iterator for Iter<'a> {
	type Item = bool;

	fn next(&mut self) -> Option<bool> {
		let result = self.bits.get(self.index);
		self.index += 1;
		result
	}
}

impl IntoIterator for BitVector {
	type Item = bool;
	type IntoIter = IntoIter;

	fn into_iter(self) -> IntoIter {
		IntoIter { bits: self, index: 0 }
	}
}

impl<'a> IntoIterator for &'a BitVector {
	type Item = bool;
	type IntoIter = Iter<'a>;

	fn into_iter(self) -> Iter<'a> {
		Iter { bits: self, index: 0 }
	}
}

pub struct Bytes<'a> {
	bits: &'a BitVector,
	index: usize,
	current_word: usize,
}

impl<'a> Iterator for Bytes<'a> {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		if self.index >= self.bits.len { return None }

		let word_offset = BitVector::to_word_offset(self.index);
		if word_offset == 0 {
			self.current_word = self.bits.words[BitVector::to_word_index(self.index)]
		}
		self.index += 8;
		Some((self.current_word >> word_offset) as u8)
	}
}