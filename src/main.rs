use std::collections::HashMap;
use std::hash::Hash;

fn bytes_from_hex(hstr: &str) -> Vec<u8> {
	hex::decode(hstr).expect("Error decoding hex")
}

fn bytes_from_base64(estr: &str) -> Vec<u8> {
	use base64::Engine;
	let stripped: String = estr.chars().filter(|c| !c.is_whitespace()).collect();
	base64::engine::general_purpose::STANDARD.decode(stripped)
		.expect("Error decoding base64")
}

fn bytes_from_str(s: &str) -> Vec<u8> {
	s.to_owned().into_bytes()
}

fn bytes_to_hex(bs: &[u8]) -> String {
	hex::encode(bs)
}

fn bytes_to_base64(bs: &[u8]) -> String {
	use base64::Engine;
	base64::engine::general_purpose::STANDARD.encode(bs)
}

fn bytes_to_string(bs: &[u8]) -> String {
	String::from_utf8_lossy(bs).into_owned()
}

fn sha256str(bs: &[u8]) -> String {
	let digest = openssl::hash::hash(openssl::hash::MessageDigest::sha256(), bs)
		.expect("Unable to hash");
	bytes_to_hex(&digest)
}

fn xor(b1: &[u8], b2: &[u8]) -> Vec<u8> {
	let size = std::cmp::max(b1.len(), b2.len());
	let pad1 = b2.len().saturating_sub(b1.len());
	let pad2 = b1.len().saturating_sub(b2.len());

	let mut res = Vec::with_capacity(size);
	for i in 0..size {
		if i < pad1 {
			res.push(b2[i]);
		} else if i < pad2 {
			res.push(b1[i]);
		} else {
			res.push(b1[i - pad1] ^ b2[i - pad2]);
		}
	}
	res
}

fn xor_encode(text: &[u8], key: &[u8]) -> Vec<u8> {
	let size = text.len();
	let mut encoded = Vec::with_capacity(size);
	for i in 0..size {
		encoded.push(text[i] ^ key[i % key.len()]);
	}
	encoded
}

fn counts<T, I>(iterator: T) -> HashMap<I, usize>
where T: Iterator<Item=I>, I: Eq + Hash {
	let mut map = HashMap::new();
	for e in iterator {
		*map.entry(e).or_insert(0) += 1;
	}
	map
}

fn frequencies<T, I>(iterator: T) -> HashMap<I, f64>
where T: Iterator<Item=I>, I: Eq + Hash {
	let counts = counts(iterator);
	let total: usize = counts.values().sum();
	counts.into_iter()
		.map(|kv| (kv.0, kv.1 as f64 / total as f64))
		.collect()
}

fn score_text(text: &str) -> f64 {
	// From https://en.wikipedia.org/wiki/Letter_frequency
	let english_freqs = HashMap::from([
		('a', 0.08200),
		('b', 0.01500),
		('c', 0.02800),
		('d', 0.04300),
		('e', 0.12700),
		('f', 0.02200),
		('g', 0.02000),
		('h', 0.06100),
		('i', 0.07000),
		('j', 0.00150),
		('k', 0.00770),
		('l', 0.04000),
		('m', 0.02400),
		('n', 0.06700),
		('o', 0.07500),
		('p', 0.01900),
		('q', 0.00095),
		('r', 0.06000),
		('s', 0.06300),
		('t', 0.09100),
		('u', 0.02800),
		('v', 0.00980),
		('w', 0.02400),
		('x', 0.00150),
		('y', 0.02000),
		('z', 0.00074),
	]);

	let text_lower = text.to_lowercase();
	let freqs = frequencies(text_lower.chars());

	let mut score = 0.0;
	for (c, f) in freqs {
		if c == ' ' { continue; }
		let fscore = english_freqs.get(&c)
			.map(|ef| ef * f * (1.0 - (ef - f).abs().sqrt()))
			.unwrap_or(f * f * -1.0);
		score += fscore;
	}

	score
}

fn hamming_distance(str1: &[u8], str2: &[u8]) -> usize {
	let mut distance = 0;
	for (b1, b2) in std::iter::zip(str1.as_ref(), str2.as_ref()) {
		distance += (b1 ^ b2).count_ones() as usize;
	}
	distance
}

fn chunked_average_distance(slice: &[u8], chunk_size: usize) -> f64 {
	let blocks: Vec<&[u8]> = slice.chunks(chunk_size).collect();
	let mut total_distance = 0.0;
	let mut num_comparisons = 0;
	for i in 0..blocks.len() {
		let block1 = blocks[i];
		for j in (i+1)..blocks.len() {
			let block2 = blocks[j];
			total_distance += hamming_distance(&block1, &block2) as f64 / usize::min(block1.len(), block2.len()) as f64;
			num_comparisons += 1
		}
	}
	total_distance / num_comparisons as f64
}

fn solve_xor<F: Fn(&str) -> f64>(ciphertext: &[u8], keysize: usize, scorer: F) -> (Vec<u8>, Vec<u8>, f64) {
	if keysize == 0 {
		return (vec![], vec![], scorer(""));
	}

	fn all_keys(size: usize) -> Vec<Vec<u8>> {
		if size == 0 {
			vec!(vec!())
		} else {
			all_keys(size - 1).iter()
				.flat_map(|k| (0..=255).map(|n| {
					let mut k2 = k.clone();
					k2.push(n);
					k2
				}))
				.collect()
		}
	}

	let mut scored: Vec<_> = all_keys(keysize).into_iter().map(|key| {
		let plaintext = xor_encode(ciphertext, &key);
		let score = scorer(&bytes_to_string(&plaintext));
		(key, plaintext, score)
	}).collect();
	scored.sort_by(|kts1, kts2| kts1.2.total_cmp(&kts2.2));

	let (key, plaintext, score) = scored.pop().unwrap();
	(key, plaintext, score)
}

fn main() {
	{ // Set 1 Challenge 1
		let num = bytes_from_hex("49276d206b696c6c696e6720796f757220627261696e206c696b65206120706f69736f6e6f7573206d757368726f6f6d");
		assert_eq!("SSdtIGtpbGxpbmcgeW91ciBicmFpbiBsaWtlIGEgcG9pc29ub3VzIG11c2hyb29t", bytes_to_base64(&num));
		println!("Set 1 Challenge 1: {}", bytes_to_base64(&num));
	}

	{ // Set 1 Challenge 2
		let res = xor(&bytes_from_hex("1c0111001f010100061a024b53535009181c"), &bytes_from_hex("686974207468652062756c6c277320657965"));
		assert_eq!("746865206b696420646f6e277420706c6179", bytes_to_hex(&res));
		println!("Set 1 Challenge 2: {}", bytes_to_hex(&res));
	}

	{ // Set 1 Challenge 3
		let ciphertext = bytes_from_hex("1b37373331363f78151b7f2b783431333d78397828372d363c78373e783a393b3736");
		let (key, text, _score) = solve_xor(&ciphertext, 1, score_text);
		println!("Set 1 Challenge 3: {} (key 0x{})", bytes_to_string(&text), bytes_to_hex(&key));
	}

	{ // Set 1 Challenge 4
		let f = std::fs::read_to_string("4.txt").unwrap();
		let mut scored: Vec<_> = f.lines()
			.map(bytes_from_hex)
			.enumerate().map(|(line_no, line)| (line_no, solve_xor(&line, 1, score_text)))
			.collect();
		scored.sort_by(|l_kts1, l_kts2| l_kts1.1.2.total_cmp(&l_kts2.1.2));
		let (line_no, (key, text, _score)) = scored.pop().unwrap();
		println!("Set 1 Challenge 4: {} (line {}, key 0x{})", bytes_to_string(&text).trim(), line_no + 1, bytes_to_hex(&key));
	}

	{ // Set 1 Challenge 5
		let plaintext = bytes_from_str("Burning 'em, if you ain't quick and nimble\nI go crazy when I hear a cymbal");
		let key = bytes_from_str("ICE");
		let ciphertext = xor_encode(&plaintext, &key);
		println!("Set 1 Challenge 5: {}", bytes_to_hex(&ciphertext));
		assert_eq!("0b3637272a2b2e63622c2e69692a23693a2a3c6324202d623d63343c2a26226324272765272a282b2f20430a652e2c652a3124333a653e2b2027630c692b20283165286326302e27282f", bytes_to_hex(&ciphertext));
	}

	{ // Set 1 Challenge 6
		assert_eq!(37, hamming_distance(&bytes_from_str("this is a test"), &bytes_from_str("wokka wokka!!!")));
		let bs = bytes_from_base64(&std::fs::read_to_string("6.txt").unwrap());
		let mut keysize_dists: Vec<_> = (2..=40).map(|keysize| {
			let block_count = 4;
			let blocks: Vec<_> = bs.chunks(keysize).take(block_count).collect();
			let mut total_distance = 0;
			for i in 0..block_count {
				for j in 0..block_count {
					total_distance += hamming_distance(blocks[i], blocks[j]);
				}
			}
			let normalized_distance = total_distance as f64 / keysize as f64;
			(keysize, normalized_distance)
		}).collect();
		keysize_dists.sort_by(|kd1, kd2| kd1.1.total_cmp(&kd2.1).reverse());
		let probable_keysize = keysize_dists.pop().unwrap().0;

		let blocks: Vec<Vec<u8>> = bs.chunks(probable_keysize).map(|c| c.to_owned()).collect();
		let transposed: Vec<_> = (0..probable_keysize).map(|n| {
			let vslice: Vec<u8> = blocks.iter().filter_map(|b| b.get(n).copied()).collect();
			let (key, text, _score) = solve_xor(&vslice, 1, score_text);
			(key, text)
		}).collect();
		let (key_t, text_t): (Vec<Vec<u8>>, Vec<Vec<u8>>) = transposed.into_iter().unzip();

		let key: Vec<u8> = key_t.into_iter().flatten().collect();
		let text = (0..)
			.map(|n| text_t.iter().filter_map(|t| t.get(n).copied()).collect::<Vec<_>>())
			.take_while(|b| !b.is_empty())
			.flatten()
			.collect::<Vec<_>>();

		println!("Set 1 Challenge 6: {} (key 0x{})", bytes_to_string(&text).lines().next().unwrap().trim(), bytes_to_hex(&key));
		assert_eq!("24df84533fc2778495577c844bcf3fe1d4d17c68d8c5cbc5a308286db58c69b6", sha256str(&text));
	}

	{ // Set 1 Challenge 7
		let bs = bytes_from_base64(&std::fs::read_to_string("7.txt").unwrap());
		let cipher = openssl::symm::Cipher::aes_128_ecb();

		let res = openssl::symm::decrypt(cipher, b"YELLOW SUBMARINE", None, &bs).unwrap();
		println!("Set 1 Challenge 7: {}", bytes_to_string(&res).lines().next().unwrap().trim());
		assert_eq!("24df84533fc2778495577c844bcf3fe1d4d17c68d8c5cbc5a308286db58c69b6", sha256str(&res));
	}
}
