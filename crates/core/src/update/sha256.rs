const BLOCK_BYTES: usize = 64;
const WORD_BYTES: usize = 4;
const LENGTH_BYTES: usize = 8;
const PADDING_MARK: u8 = 0x80;
const BITS_PER_BYTE: u64 = 8;
const SCHEDULE_WORDS: usize = 64;

const INITIAL_STATE: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const ROUND_CONSTANTS: [u32; SCHEDULE_WORDS] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn of(data: &[u8]) -> Sha256Digest {
        let mut state = INITIAL_STATE;
        let (blocks, remainder) = data.as_chunks::<BLOCK_BYTES>();
        let tail = final_blocks(remainder, data.len());
        for block in blocks.iter().chain(tail.as_chunks::<BLOCK_BYTES>().0) {
            compress(&mut state, block);
        }
        let mut digest = [0u8; 32];
        for (bytes, word) in digest.as_chunks_mut::<WORD_BYTES>().0.iter_mut().zip(state) {
            bytes.copy_from_slice(&word.to_be_bytes());
        }
        Sha256Digest(digest)
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

fn final_blocks(remainder: &[u8], total_length: usize) -> Vec<u8> {
    let bit_length = (total_length as u64).wrapping_mul(BITS_PER_BYTE);
    let mut message = Vec::with_capacity(2 * BLOCK_BYTES);
    message.extend_from_slice(remainder);
    message.push(PADDING_MARK);
    while message.len() % BLOCK_BYTES != BLOCK_BYTES - LENGTH_BYTES {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    message
}

fn schedule(block: &[u8; BLOCK_BYTES]) -> [u32; SCHEDULE_WORDS] {
    let mut words = [0u32; SCHEDULE_WORDS];
    for (word, bytes) in words.iter_mut().zip(block.as_chunks::<WORD_BYTES>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for index in 16..SCHEDULE_WORDS {
        let previous = words[index - 2];
        let older = words[index - 15];
        let small_one = previous.rotate_right(17) ^ previous.rotate_right(19) ^ (previous >> 10);
        let small_zero = older.rotate_right(7) ^ older.rotate_right(18) ^ (older >> 3);
        words[index] = small_one
            .wrapping_add(words[index - 7])
            .wrapping_add(small_zero)
            .wrapping_add(words[index - 16]);
    }
    words
}

fn compress(state: &mut [u32; 8], block: &[u8; BLOCK_BYTES]) {
    let words = schedule(block);
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (word, constant) in words.iter().zip(ROUND_CONSTANTS) {
        let big_one = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ (!e & g);
        let first = h
            .wrapping_add(big_one)
            .wrapping_add(choice)
            .wrapping_add(constant)
            .wrapping_add(*word);
        let big_zero = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = big_zero.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}
