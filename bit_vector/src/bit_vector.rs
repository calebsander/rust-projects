use std::iter::{FromIterator, IntoIterator};
use std::mem;
use std::ptr;

pub struct BitVector {
	len: usize,
	words: Vec<usize>,
}

const WORD_BITS: usize = mem::size_of::<usize>() * 8;
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
	pub fn fill(&mut self, value: bool) {
		let filled_word = Self::fill_word(value);
		for word in &mut self.words[..Self::to_words_ceil(self.len)] {
			*word = filled_word
		}
	}
	pub fn get(&self, index: usize) -> Option<bool> {
		if index >= self.len { return None }

		Some(unsafe { self.get_unchecked(index) })
	}
	pub unsafe fn get_unchecked(&self, index: usize) -> bool {
		let word_index = Self::to_word_index(index);
		let word_offset = Self::to_word_offset(index);
		*self.words.get_unchecked(word_index) >> word_offset & 1 > 0
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
		Some(unsafe { self.get_unchecked(self.len) })
	}
	pub fn push(&mut self, value: bool) {
		let word_index = Self::to_word_index(self.len);
		if word_index == self.words.len() {
			self.words.push(value as usize)
		}
		else {
			unsafe { self.set_unchecked(self.len, value) }
		}
		self.len += 1;
	}
	pub fn set(&mut self, index: usize, value: bool) -> Option<()> {
		if index >= self.len { return None }

		unsafe { self.set_unchecked(index, value) }
		Some(())
	}
	pub unsafe fn set_unchecked(&mut self, index: usize, value: bool) {
		let word = self.words.get_unchecked_mut(Self::to_word_index(index));
		let set_bit = 1 << Self::to_word_offset(index);
		*word = *word & !set_bit | Self::fill_word(value) & set_bit;
	}

	pub fn bytes(&self) -> Bytes {
		Bytes {
			bit_index: 0,
			bit_len: self.len,
			current_word: if self.is_empty() { ptr::null() } else { self.words.as_ptr() }
		}
	}

	fn to_word_index(bit_index: usize) -> usize {
		bit_index >> LOG_WORD_BITS
	}
	fn to_word_offset(bit_index: usize) -> u8 {
		(bit_index & (WORD_BITS - 1)) as u8
	}
	fn to_words_ceil(bits: usize) -> usize {
		Self::to_word_index(bits) + (Self::to_word_offset(bits) > 0) as usize
	}
	fn from_word_index(word_index: usize) -> usize {
		word_index << LOG_WORD_BITS
	}
	fn fill_word(value: bool) -> usize {
		-(value as isize) as usize
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
		Self::from_iter(iter.into_iter().cloned())
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

pub struct Bytes {
	bit_index: usize,
	bit_len: usize,
	current_word: *const usize,
}

impl Iterator for Bytes {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		if self.current_word.is_null() { return None }

		let word_offset = BitVector::to_word_offset(self.bit_index);
		let mut byte;
		unsafe {
			if self.bit_index > 0 && word_offset == 0 {
				self.current_word = self.current_word.add(1)
			}
			byte = (*self.current_word >> word_offset) as u8;
		}
		self.bit_index += 8;
		let unset_bits = self.bit_index as isize - self.bit_len as isize;
		if unset_bits >= 0 {
			byte = byte << unset_bits >> unset_bits;
			self.current_word = ptr::null();
		}
		Some(byte)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::iter;

	#[test]
	fn test_get_set() {
		let mut bit_vec = BitVector::from_iter(iter::repeat(false).take(1000));
		for i in 1000..2000 { assert_eq!(bit_vec.set(i, false), None) }
		for i in 0..1000 {
			let fill_value = i & 1 > 0;
			for j in 0..1000 {
				assert_eq!(bit_vec.set(j, (j == i) ^ fill_value), Some(()))
			}
			for j in 0..1000 {
				assert_eq!(bit_vec.get(j), Some((j == i) ^ fill_value))
			}
		}
	}

	#[test]
	fn test_push_pop() {
		for one_spacing in 1..100 {
			let mut bit_vec = BitVector::new();
			for i in 0..1000 {
				assert_eq!(bit_vec.len(), i);
				bit_vec.push(i % one_spacing == 0);
			}
			for i in 0..1000 {
				assert_eq!(bit_vec.get(i), Some(i % one_spacing == 0))
			}
			for i in 1000..1100 { assert_eq!(bit_vec.get(i), None) }
			for i in (0..1000).rev() {
				assert_eq!(bit_vec.pop(), Some(i % one_spacing == 0))
			}
			for _ in 0..100 { assert_eq!(bit_vec.pop(), None) }
		}
	}

	#[test]
	fn test_fill() {
		let mut bit_vec = BitVector::from_iter(iter::repeat(false).take(100000));
		assert_eq!(bit_vec.len(), 100000);
		for i in 0..100000 { assert_eq!(bit_vec.get(i), Some(false)) }
		bit_vec.fill(true);
		assert_eq!(bit_vec.len(), 100000);
		for i in 0..100000 { assert_eq!(bit_vec.get(i), Some(true)) }
		bit_vec.fill(false);
		assert_eq!(bit_vec.len(), 100000);
		for i in 0..100000 { assert_eq!(bit_vec.get(i), Some(false)) }
	}

	#[test]
	fn test_bytes() {
		// Test partial bytes
		let mut bit_vec = BitVector::new();
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), []);
		bit_vec.push(true);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b1]);
		bit_vec.push(false);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b01]);
		bit_vec.push(true);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b101]);
		bit_vec.push(false);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b0101]);
		bit_vec.push(true);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b10101]);
		bit_vec.push(false);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b010101]);
		bit_vec.push(true);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b1010101]);
		bit_vec.push(false);
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b01010101]);
		bit_vec.pop();
		bit_vec.pop();
		assert_eq!(bit_vec.bytes().collect::<Vec<_>>(), [0b010101]);

		// Test multiple bytes and words
		bit_vec.clear();
		for i in 0..=255 {
			let mut byte = i as u8;
			for _ in 0..8 {
				bit_vec.push(byte & 1 > 0);
				byte >>= 1;
			}
			assert!(bit_vec.bytes().eq(0..=i));
		}
	}
}