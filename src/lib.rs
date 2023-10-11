use std::num::NonZeroU8;

static UTF8_SJIS: phf::Map<char, [u8; 2]> = include!(concat!(env!("OUT_DIR"), "/utf8sjis.rs"));
static SJIS_UTF8: [[char; 94]; 94] = include!(concat!(env!("OUT_DIR"), "/sjisutf8.rs"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedChar {
	One(u8),
	Two(u8, NonZeroU8),
	// This NZU8 is a bit awkward, but it adds a niche, bringing size down to 2 bytes instead of 3.
	// Premature optimization, I know.
}

impl IntoIterator for EncodedChar {
	type Item = u8;
	type IntoIter = std::array::IntoIter<u8, 2>;
	fn into_iter(self) -> Self::IntoIter {
		match self {
			EncodedChar::One(a) => {
				let mut it = [0, a].into_iter();
				it.next();
				it
			}
			EncodedChar::Two(a, b) => [a, b.into()].into_iter(),
		}
	}
}

pub fn encode_char(char: char) -> Option<EncodedChar> {
	if char.is_ascii() {
		Some(EncodedChar::One(char as u8))
	} else if ('｡'..='ﾟ').contains(&char) {
		Some(EncodedChar::One((char as u32 - '｡' as u32) as u8 + 0xA1))
	} else if let Some(&[k1, k2]) = UTF8_SJIS.get(&char) {
		Some(EncodedChar::Two(k1, k2.try_into().unwrap()))
	} else {
		None
	}
}

pub fn decode_char(iter: &mut impl Iterator<Item=u8>) -> Option<char> {
	let a = match iter.next()? {
		a@0x00..=0x7F => return Some(char::from(a)),
		a@0xA1..=0xDF => return Some(char::from_u32('｡' as u32 + (a - 0xA1) as u32).unwrap()),
		a@0x81..=0x9F => a - 0x81,
		a@0xE0..=0xEF => a - 0xE0 + 0x1F,
		0x80 | 0xA0 | 0xF0.. => return None
	} as usize;
	let b = match iter.next()? {
		b@0x40..=0x7E => b - 0x40,
		b@0x80..=0xFC => b - 0x80 + 0x3F,
		..=0x3F | 0x7F | 0xFD.. => return None
	} as usize;
	Some(SJIS_UTF8[a*2+b/94][b%94])
}

#[test]
fn encode_then_decode() {
	for char in (0..=0xFFFF).filter_map(char::from_u32) {
		if let Some(enc) = encode_char(char) {
			assert_eq!(decode_char(&mut enc.into_iter()), Some(char))
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
		if let Some(dec) = decode_char(&mut it) {
			let consumed = &array[..2-it.as_slice().len()];
			let enc = encode_char(dec).unwrap();
			let enc = enc.into_iter().collect::<Vec<u8>>();
			if enc != consumed && dec != '・' && !duplicates.contains(&array) {
				panic!("{:02X?} {:?} {:02X?}", consumed, dec, enc);
			};
		}
	}
}
