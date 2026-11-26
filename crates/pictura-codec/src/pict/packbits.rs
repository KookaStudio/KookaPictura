//! PackBits (Apple RLE) for PICT scanlines.
//!
//! The PICT form prefixes each compressed scanline with its byte count; these
//! helpers handle only the compressed bytes. A control byte `0..=127` is a
//! literal run of `control + 1` bytes, `-1..=-127` repeats the next byte
//! `1 - control` times, and `-128` is a no-op.

/// Decode PackBits bytes into `out` until it holds `expected` bytes, returning
/// the number of input bytes consumed. A truncated run is an error.
pub(crate) fn decode_exact(input: &[u8], out: &mut Vec<u8>, expected: usize) -> Result<usize, ()> {
    let mut at = 0;
    while out.len() < expected {
        let control = *input.get(at).ok_or(())? as i8;
        at += 1;
        if control >= 0 {
            let count = control as usize + 1;
            let end = at.checked_add(count).ok_or(())?;
            let bytes = input.get(at..end).ok_or(())?;
            let take = (expected - out.len()).min(count);
            out.extend_from_slice(&bytes[..take]);
            at = end;
        } else if control != -128 {
            let count = (1 - isize::from(control)) as usize;
            let byte = *input.get(at).ok_or(())?;
            at += 1;
            let take = (expected - out.len()).min(count);
            out.extend(std::iter::repeat_n(byte, take));
        }
    }
    Ok(at)
}

/// Encode `input` as PackBits. Literal runs stop before a three-byte repeat and
/// never exceed 128 bytes; repeat runs never exceed 128 bytes.
pub(crate) fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < input.len() {
        let mut run = 1;
        while at + run < input.len() && input[at + run] == input[at] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            out.push((257 - run) as u8);
            out.push(input[at]);
            at += run;
            continue;
        }
        let start = at;
        at += run;
        while at < input.len() && at - start < 128 {
            if at + 2 < input.len() && input[at] == input[at + 1] && input[at] == input[at + 2] {
                break;
            }
            at += 1;
        }
        let count = at - start;
        out.push((count - 1) as u8);
        out.extend_from_slice(&input[start..at]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(bytes: &[u8]) {
        let packed = encode(bytes);
        let mut out = Vec::new();
        let used = decode_exact(&packed, &mut out, bytes.len()).unwrap();
        assert_eq!(used, packed.len());
        assert_eq!(out, bytes);
    }

    #[test]
    fn literal_and_repeat_runs_round_trip() {
        round_trip(&[1, 2, 3, 4, 5]);
        round_trip(&[9, 9, 9, 9, 9]);
        round_trip(&[0, 1, 1, 1, 2, 3, 3, 3, 3, 4]);
        round_trip(&vec![7; 300]);
        round_trip(&(0..=255).collect::<Vec<u8>>());
    }

    #[test]
    fn no_op_control_is_skipped() {
        let mut out = Vec::new();
        let used = decode_exact(&[0x80, 0x00, 0x2a], &mut out, 1).unwrap();
        assert_eq!(used, 3);
        assert_eq!(out, vec![0x2a]);
    }

    #[test]
    fn truncated_input_is_an_error() {
        let mut out = Vec::new();
        assert!(decode_exact(&[0x03, 1, 2], &mut out, 4).is_err());
        assert!(decode_exact(&[0xfe], &mut out, 4).is_err());
    }
}
