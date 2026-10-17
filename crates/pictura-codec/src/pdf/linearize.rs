//! Optimize For Fast Web View: a linearized one-page file (ISO 32000-1
//! Annex F; design D10). Layout: header, linearization dictionary, first-page
//! cross-reference section, catalog (+ `/Encrypt`), primary hint stream, the
//! first page's objects, then every other object and the main
//! cross-reference section. The first-page part takes the high object
//! numbers. Offsets in the hint tables are given as if the hint stream were
//! absent, as readers (and `qpdf --check-linearization`) expect.

use super::object::{self, Body, Dict, Obj, Objects};
use super::{header, trailer_entries, Built};

fn remap(obj: &Obj, map: &[u32]) -> Obj {
    match obj {
        Obj::Ref(n) => Obj::Ref(map[*n as usize]),
        Obj::Array(items) => Obj::Array(items.iter().map(|o| remap(o, map)).collect()),
        Obj::Dict(dict) => Obj::Dict(Dict(
            dict.0
                .iter()
                .map(|(k, v)| (k.clone(), remap(v, map)))
                .collect(),
        )),
        other => other.clone(),
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    used: u32,
}

impl BitWriter {
    fn push(&mut self, value: u64, bits: u32) {
        for i in (0..bits).rev() {
            if self.used == 0 {
                self.bytes.push(0);
            }
            if value >> i & 1 == 1 {
                *self.bytes.last_mut().expect("pushed") |= 0x80 >> self.used;
            }
            self.used = (self.used + 1) % 8;
        }
    }

    fn align(&mut self) {
        self.used = 0;
    }
}

fn bits_for(value: u64) -> u32 {
    64 - value.leading_zeros()
}

/// The values the layout feeds back into itself.
#[derive(Debug, Default, Clone, PartialEq)]
struct Layout {
    length: usize,
    hint: (usize, usize),
    end_of_first_page: usize,
    main_xref: usize,
    main_xref_first_entry: usize,
    first_xref: usize,
    offsets: Vec<usize>,
    hint_data: Vec<u8>,
}

/// The primary hint stream's data and its shared object table offset.
fn hint_data(
    layout: &Layout,
    page: u32,
    content: u32,
    page_objects: &[u32],
    lengths: &[usize],
) -> (Vec<u8>, usize) {
    let hint_len = layout.hint.1;
    let page_offset = layout.offsets[page as usize] - hint_len;
    let page_length = layout.end_of_first_page - layout.offsets[page as usize];
    let content_offset = layout.offsets[content as usize] - layout.offsets[page as usize];
    let content_length = lengths[content as usize];
    let mut bits = BitWriter::default();
    // Page offset hint table header (Table F.3); one page, so every delta is 0.
    bits.push(page_objects.len() as u64, 32);
    bits.push(page_offset as u64, 32);
    bits.push(0, 16);
    bits.push(page_length as u64, 32);
    bits.push(0, 16);
    bits.push(content_offset as u64, 32);
    bits.push(0, 16);
    bits.push(content_length as u64, 32);
    bits.push(0, 16);
    bits.push(0, 16);
    bits.push(0, 16);
    bits.push(0, 16);
    bits.push(1, 16);
    bits.align();
    let shared_offset = bits.bytes.len();
    // Shared object hint table (Table F.5): one entry per first-page object.
    let group_lengths: Vec<u64> = page_objects
        .iter()
        .map(|&n| lengths[n as usize] as u64)
        .collect();
    let least = group_lengths.iter().copied().min().unwrap_or(0);
    let width = bits_for(group_lengths.iter().map(|l| l - least).max().unwrap_or(0));
    bits.push(0, 32);
    bits.push(0, 32);
    bits.push(page_objects.len() as u64, 32);
    bits.push(page_objects.len() as u64, 32);
    bits.push(0, 16);
    bits.push(least, 32);
    bits.push(u64::from(width), 16);
    bits.align();
    for length in &group_lengths {
        bits.push(length - least, width);
    }
    bits.align();
    for _ in &group_lengths {
        bits.push(0, 1);
    }
    bits.align();
    (bits.bytes, shared_offset)
}

/// Append object `num`, recording its offset and length.
fn emit(
    out: &mut Vec<u8>,
    objects: &Objects,
    num: u32,
    cipher: Option<object::Cipher>,
    offsets: &mut [usize],
    lengths: &mut [usize],
) {
    offsets[num as usize] = out.len();
    let bytes = objects.serialize(num, cipher);
    lengths[num as usize] = bytes.len();
    out.extend(bytes);
}

pub(crate) fn write(built: &Built) -> Vec<u8> {
    let old_total = built.objects.count();
    // The thumbnail stays out of the first-page section (readers treat it as
    // its own part).
    let thumbnail = match built.objects.body(built.page) {
        Body::Obj(Obj::Dict(page)) => match page.get("Thumb") {
            Some(Obj::Ref(n)) => Some(*n),
            _ => None,
        },
        _ => None,
    };
    let page_objects: Vec<u32> = built
        .page_objects
        .iter()
        .copied()
        .filter(|&n| Some(n) != thumbnail)
        .collect();
    let mut first_part = vec![built.catalog];
    first_part.extend(built.encrypt);
    let rest: Vec<u32> = (1..=old_total)
        .filter(|n| !first_part.contains(n) && !page_objects.contains(n))
        .collect();
    let k = rest.len() as u32;
    let lin = k + 1;
    let mut map = vec![0u32; old_total as usize + 1];
    for (i, &n) in rest.iter().enumerate() {
        map[n as usize] = i as u32 + 1;
    }
    let mut next = lin + 1;
    for &n in &first_part {
        map[n as usize] = next;
        next += 1;
    }
    let hint = next;
    next += 1;
    for &n in &page_objects {
        map[n as usize] = next;
        next += 1;
    }
    let total = next - 1;

    let mut objects = Objects::default();
    for _ in 1..=total {
        objects.reserve();
    }
    for old in 1..=old_total {
        let new = map[old as usize];
        let body = match built.objects.body(old) {
            Body::Obj(obj) => Body::Obj(remap(obj, &map)),
            Body::Stream(dict, data) => match remap(&Obj::Dict(dict.clone()), &map) {
                Obj::Dict(dict) => Body::Stream(dict, data.clone()),
                _ => unreachable!("a dictionary remaps to a dictionary"),
            },
        };
        objects.set(new, body);
        if built.objects.is_plain(old) {
            objects.mark_plain(new);
        }
    }
    objects.mark_plain(lin);
    let page = map[built.page as usize];
    let content = map[built.content as usize];
    let new_page_objects: Vec<u32> = page_objects.iter().map(|&n| map[n as usize]).collect();
    let mapped = |n: u32| map[n as usize];

    let cipher = built
        .encryptor
        .as_ref()
        .map(|e| move |num: u32, data: &[u8]| e.encrypt(num, data));
    let cipher: Option<object::Cipher> = cipher.as_ref().map(|c| c as object::Cipher);

    let mut layout = Layout {
        offsets: vec![0; total as usize + 1],
        ..Layout::default()
    };
    let mut shared_offset = 0;
    let mut out = Vec::new();
    for _ in 0..8 {
        let mut next_layout = Layout {
            offsets: vec![0; total as usize + 1],
            ..Layout::default()
        };
        let mut lengths = vec![0usize; total as usize + 1];
        objects.set(
            lin,
            Body::Obj(
                Dict::new()
                    .with("Linearized", Obj::Int(1))
                    .with("L", Obj::Int(layout.length as i64))
                    .with(
                        "H",
                        Obj::Array(vec![
                            Obj::Int(layout.hint.0 as i64),
                            Obj::Int(layout.hint.1 as i64),
                        ]),
                    )
                    .with("O", Obj::Int(i64::from(page)))
                    .with("E", Obj::Int(layout.end_of_first_page as i64))
                    .with("N", Obj::Int(1))
                    .with("T", Obj::Int(layout.main_xref_first_entry as i64))
                    .into(),
            ),
        );
        objects.set(
            hint,
            Body::Stream(
                Dict::new().with("S", Obj::Int(shared_offset as i64)),
                layout.hint_data.clone(),
            ),
        );
        out = header(built.minor);
        let offsets = &mut next_layout.offsets;
        emit(&mut out, &objects, lin, cipher, offsets, &mut lengths);
        next_layout.first_xref = out.len();
        out.extend(object::xref_section(lin, &layout.offsets[lin as usize..]));
        let trailer = trailer_entries(built, &mapped)
            .with("Size", Obj::Int(i64::from(total) + 1))
            .with("Prev", Obj::Int(layout.main_xref as i64));
        out.extend_from_slice(b"trailer\n");
        object::write_obj(&mut out, &trailer.into(), 0, None);
        out.extend_from_slice(b"\nstartxref\n0\n%%EOF\n");
        let offsets = &mut next_layout.offsets;
        for n in lin + 1..hint {
            emit(&mut out, &objects, n, cipher, offsets, &mut lengths);
        }
        let hint_start = out.len();
        emit(&mut out, &objects, hint, cipher, offsets, &mut lengths);
        next_layout.hint = (hint_start, out.len() - hint_start);
        let offsets = &mut next_layout.offsets;
        for &n in &new_page_objects {
            emit(&mut out, &objects, n, cipher, offsets, &mut lengths);
        }
        next_layout.end_of_first_page = out.len();
        let offsets = &mut next_layout.offsets;
        for n in 1..=k {
            emit(&mut out, &objects, n, cipher, offsets, &mut lengths);
        }
        next_layout.main_xref = out.len();
        let section = object::xref_section(1, &next_layout.offsets[1..=k as usize]);
        next_layout.main_xref_first_entry =
            next_layout.main_xref + format!("xref\n0 {}\n", k + 1).len() - 1;
        out.extend(section);
        out.extend_from_slice(
            format!(
                "trailer\n<</Size {}>>\nstartxref\n{}\n%%EOF\n",
                k + 1,
                next_layout.first_xref
            )
            .as_bytes(),
        );
        next_layout.length = out.len();
        let (data, shared) = hint_data(&next_layout, page, content, &new_page_objects, &lengths);
        next_layout.hint_data = data;
        shared_offset = shared;
        if next_layout == layout {
            break;
        }
        layout = next_layout;
    }
    out
}
