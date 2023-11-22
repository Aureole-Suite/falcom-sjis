static UTF8_SJIS: phf::Map<char, [u8; 2]> = include!(concat!(env!("OUT_DIR"), "/utf8sjis.rs"));
static SJIS_UTF8: [[char; 94]; 94] = include!(concat!(env!("OUT_DIR"), "/sjisutf8.rs"));

/// An encoded character in Shift JIS encoding.
///
/// This represents either one or two bytes, and is most conveniently used via its `IntoIterator` impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedChar {
	One([u8; 1]),
	Two([u8; 2]),
}

impl EncodedChar {
	/// The replacement character used with [`encode_lossy`], namely `・`.
	pub const REPLACEMENT: EncodedChar = EncodedChar::Two([0x81, 0x45]);
}

impl IntoIterator for EncodedChar {
	type Item = u8;
	type IntoIter = std::array::IntoIter<u8, 2>;
	fn into_iter(self) -> Self::IntoIter {
		match self {
			EncodedChar::One([a]) => {
				let mut it = [0, a].into_iter();
				it.next();
				it
			}
			EncodedChar::Two([a, b]) => [a, b].into_iter(),
		}
	}
}

/// Encodes a single character, yielding either an error or one or two bytes.
pub fn encode_char(char: char) -> Option<EncodedChar> {
	if char.is_ascii() {
		Some(EncodedChar::One([char as u8]))
	} else if ('｡'..='ﾟ').contains(&char) {
		Some(EncodedChar::One([(char as u32 - '｡' as u32) as u8 + 0xA1]))
	} else if let Some(&[k1, k2]) = UTF8_SJIS.get(&char) {
		Some(EncodedChar::Two([k1, k2]))
	} else {
		None
	}
}

/// Decodes a single character from the input.
///
/// Consumes one or two bytes from the iterator and returns the decoding result as per [`encode_char_from`], or `None` if the iterator is empty.
pub fn decode_char(iter: &mut impl Iterator<Item = u8>) -> Option<Result<char, EncodedChar>> {
	iter.next().map(|b1| decode_char_from(b1, || iter.next()))
}

/// Decodes a single character from an already-read input.
///
/// It will call the `b2` closure if necessary to complete a two-byte sequence.
pub fn decode_char_from(b1: u8, b2: impl FnOnce() -> Option<u8>) -> Result<char, EncodedChar> {
	let enc = EncodedChar::One([b1]);
	let a = match b1 {
		a @ 0x00..=0x7F => return Ok(char::from(a)),
		a @ 0xA1..=0xDF => return Ok(char::from_u32('｡' as u32 + (a - 0xA1) as u32).unwrap()),
		a @ 0x81..=0x9F => a - 0x81,
		a @ 0xE0..=0xEF => a - 0xE0 + 0x1F,
		0x80 | 0xA0 | 0xF0.. => return Err(enc),
	} as usize;

	let b2 = b2().ok_or(enc)?;
	let enc = EncodedChar::Two([b1, b2]);
	let b = match b2 {
		b @ 0x40..=0x7E => b - 0x40,
		b @ 0x80..=0xFC => b - 0x80 + 0x3F,
		..=0x3F | 0x7F | 0xFD.. => return Err(enc),
	} as usize;

	let ch = SJIS_UTF8[a * 2 + b / 94][b % 94];
	if ch == '�' {
		return Err(enc);
	}
	Ok(ch)
}

#[test]
fn encode_replacement() {
	assert_eq!(EncodedChar::REPLACEMENT, encode_char('・').unwrap())
}

#[test]
fn encode_then_decode() {
	for char in (0..=0xFFFF).filter_map(char::from_u32) {
		if let Some(enc) = encode_char(char) {
			assert_eq!(decode_char(&mut enc.into_iter()), Some(Ok(char)))
		}
	}
}

#[test]
fn decode_then_encode() {
	let duplicates = [
		[0x87, 0x90], // ≒  maps to [81, E0]
		[0x87, 0x91], // ≡  maps to [81, DF]
		[0x87, 0x92], // ∫  maps to [81, E7]
		[0x87, 0x95], // √  maps to [81, E3]
		[0x87, 0x96], // ⊥  maps to [81, DB]
		[0x87, 0x97], // ∠  maps to [81, DA]
		[0x87, 0x9A], // ∵  maps to [81, E6]
		[0x87, 0x9B], // ∩  maps to [81, BF]
		[0x87, 0x9C], // ∪  maps to [81, BE]
		[0xEE, 0xF9], // ￢ maps to [81, CA]
	];
	for array in (0..=0xFFFF).map(u16::to_le_bytes) {
		let mut it = array.into_iter();
		if let Some(Ok(dec)) = decode_char(&mut it) {
			let consumed = &array[..2 - it.as_slice().len()];
			let enc = encode_char(dec).unwrap();
			let enc = enc.into_iter().collect::<Vec<u8>>();
			if enc != consumed && !duplicates.contains(&array) {
				panic!("{:02X?} {:?} {:02X?}", consumed, dec, enc);
			};
		}
	}
}

/// Encodes a string into a byte vec.
///
/// Returns `Err(position)` if a codepoint cannot be represented in Shift JIS, where `position` is
/// the UTF-8 offset of the offending codepoint in the input string.
pub fn encode(str: &str) -> Result<Vec<u8>, usize> {
	let mut out = Vec::new();
	for (pos, char) in str.char_indices() {
		if let Some(char) = encode_char(char) {
			out.extend(char)
		} else {
			return Err(pos);
		}
	}
	Ok(out)
}

/// Encodes a string into a byte vec, lossily.
///
/// Characters that cannot be encoded in Shift-JIS are substituted with [`EncodedChar::REPLACEMENT`].
pub fn encode_lossy(str: &str) -> Vec<u8> {
	let mut out = Vec::new();
	for char in str.chars() {
		if let Some(char) = encode_char(char) {
			out.extend(char)
		} else {
			out.extend(EncodedChar::REPLACEMENT)
		}
	}
	out
}

#[rustfmt::skip]
#[test]
fn test_encode() {
	assert_eq!(
		encode("日本ファルコム").as_deref(),
		Ok(&[0x93u8, 0xFA, 0x96, 0x7b, 0x83, 0x74, 0x83, 0x40, 0x83, 0x8B, 0x83, 0x52, 0x83, 0x80] as &[_]),
	);
	assert_eq!(encode("日本2=₂"), Err("日本2=".len()),);
	assert_eq!(decode_lossy(&encode_lossy("日本2=₂")), "日本2=・");
}

/// Decodes a byte slice into a string.
///
/// Returns `Err(position)` on encountering an invalid byte sequence, where `position` is the
/// offset of the first byte of the sequence.
pub fn decode(input: &[u8]) -> Result<String, (usize, EncodedChar)> {
	let mut out = String::new();
	let mut pos = 0;
	let mut iter = input.iter().copied().inspect(|_| pos += 1);
	while let Some(b1) = iter.next() {
		match decode_char_from(b1, || iter.next()) {
			Ok(char) => out.push(char),
			Err(enc) => return Err((pos - enc.into_iter().len(), enc)),
		}
	}
	Ok(out)
}

/// Decodes a byte slice into a string, lossily.
///
/// Invalid bytes are replaced with the unicode replacement character, one per byte.
pub fn decode_lossy(input: &[u8]) -> String {
	let mut out = String::new();
	let mut iter = input.iter().copied();
	while let Some(b1) = iter.next() {
		match decode_char_from(b1, || iter.next()) {
			Ok(char) => out.push(char),
			Err(_) => out.push('�'),
		}
	}
	out
}

#[rustfmt::skip]
#[test]
fn test_decode() {
	assert_eq!(
		decode(&[0x93, 0xFA, 0x96, 0x7b, 0x83, 0x74, 0x83, 0x40, 0x83, 0x8B, 0x83, 0x52, 0x83, 0x80]).as_deref(),
		Ok("日本ファルコム"),
	);
	assert_eq!(
		decode_lossy(&[0x93, 0xFA, 0x96, 0x7b, 0x83, 0x74, 0x83, 0x81, 0x40, 0x83, 0x8B, 0x83, 0x52, 0x83, 0x80]),
		"日本フメ@ルコム",
	);
	assert_eq!(
		decode(&[0x93, 0xFA, 0x96, 0x7B, 0x32, 0x3D, 0x96, 0x7B, 0xEE, 0xEE, 0x83, 0x40]),
		Err((8, EncodedChar::Two([0xEE, 0xEE]))),
	);
}
