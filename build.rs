use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

use gospel::read::Reader;

fn main() -> anyhow::Result<()> {
	let out = PathBuf::from(env::var("OUT_DIR")?);

	let mut table = phf_codegen::Map::new();
	let f = &mut Reader::new(include_bytes!("utf8sjis.dat"));
	let mut dup = HashSet::new();
	for _ in 0..f.u32_le()? {
		let char = f.u32_be()?.to_le_bytes();
		let b = f.array::<2>()?;
		f.check(&[0, 0])?;
		let char = std::str::from_utf8(&char)?.chars().next().unwrap();
		// There are duplicate encodings for √∠∩∪∫∵≒≡⊥￢
		if dup.insert(char) {
			table.entry(char, &format!("{:#02X?}", b));
		}
	}
	assert!(f.remaining().is_empty());
	std::fs::write(out.join("utf8sjis.rs"), table.build().to_string())?;

	let mut table = Vec::new();
	let f = &mut Reader::new(include_bytes!("sjisutf8.dat"));
	for _ in 0..f.u32_le()? {
		let char = f.array::<4>()?;
		let char = std::str::from_utf8(&char)?.chars().next().unwrap();
		table.push(char);
	}
	assert!(f.remaining().is_empty());
	for _ in 0..188 {
		table.push('・');
	}
	let table = table.chunks(94).collect::<Vec<_>>();
	std::fs::write(out.join("sjisutf8.rs"), format!("{:?}", table))?;

	Ok(())
}
