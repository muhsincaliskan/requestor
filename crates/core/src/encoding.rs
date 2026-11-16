const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64_PADDING: char = '=';
pub const FORM_PAIR_SEPARATOR: &str = "&";
const UNRESERVED_EXTRA: &[u8] = b"-._~";

pub fn base64_encode(data: &[u8]) -> String {
    data.chunks(3).map(encode_chunk).collect()
}

fn encode_chunk(chunk: &[u8]) -> String {
    let [first, second, third] = [0, 1, 2].map(|index| chunk.get(index).copied().unwrap_or(0));
    let group = u32::from(first) << 16 | u32::from(second) << 8 | u32::from(third);
    let symbols =
        [18, 12, 6, 0].map(|shift| BASE64_ALPHABET[(group >> shift & 0x3f) as usize] as char);
    let kept = chunk.len() + 1;
    symbols
        .iter()
        .enumerate()
        .map(|(position, symbol)| {
            if position < kept {
                *symbol
            } else {
                BASE64_PADDING
            }
        })
        .collect()
}

pub fn percent_encode(text: &str) -> String {
    text.bytes()
        .fold(String::with_capacity(text.len()), |mut encoded, byte| {
            if byte.is_ascii_alphanumeric() || UNRESERVED_EXTRA.contains(&byte) {
                encoded.push(byte as char);
            } else {
                encoded.push_str(&format!("%{byte:02X}"));
            }
            encoded
        })
}

pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let (byte, consumed) = decode_at(bytes, index);
        decoded.push(byte);
        index += consumed;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn decode_at(bytes: &[u8], index: usize) -> (u8, usize) {
    match (bytes[index], bytes.get(index + 1), bytes.get(index + 2)) {
        (b'%', Some(high), Some(low)) => match (hex_value(*high), hex_value(*low)) {
            (Some(high), Some(low)) => (high << 4 | low, 3),
            _ => (b'%', 1),
        },
        (b'+', _, _) => (b' ', 1),
        (byte, _, _) => (byte, 1),
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    char::from(byte).to_digit(16).map(|digit| digit as u8)
}

#[cfg(test)]
mod tests {
    use super::{base64_encode, percent_decode, percent_encode};

    #[test]
    fn base64_matches_rfc_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
    }

    #[test]
    fn percent_decoding_reverses_encoding_and_tolerates_garbage() {
        assert_eq!(percent_decode("a%20b%26c+d"), "a b&c d");
        assert_eq!(percent_decode("%C3%BC"), "ü");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn percent_encoding_escapes_reserved_and_non_ascii() {
        assert_eq!(percent_encode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(percent_encode("ü"), "%C3%BC");
        assert_eq!(percent_encode("safe-_.~09"), "safe-_.~09");
    }
}
