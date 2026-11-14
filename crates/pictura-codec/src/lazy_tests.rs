//! Lazy, file-backed open (#1218): the eager read is the oracle.

use super::*;
use crate::{read_psd_lazy, read_psd_within_options, write_psd, ImageBudget, OpenOptions};
use pictura_core::{Compression, Document, LayerMask, Residency};

const W: u32 = 600;
const H: u32 = 1100;

fn noise(seed: u32, n: usize) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_add(seed).wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect()
}

/// A Background of seeded noise under a masked, partly transparent layer
/// that crosses several bands and a partial tile column.
fn fixture(compression: Compression) -> Document {
    let mut rgba = noise(1, (W * H * 4) as usize);
    // Runs every few rows, so RLE rows differ in length.
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        if (i / W as usize).is_multiple_of(5) {
            px[..3].copy_from_slice(&[10, 20, 30]);
        }
        px[3] = 255;
    }
    let mut doc = Document::from_rgba("Background", W, H, &rgba);
    let bottom = doc.layers.get_mut(0).expect("a layer");
    bottom.channels.retain(|c| c.id != -1);
    bottom.background = true;
    let (tw, th) = (300u32, 520u32);
    let top = Document::from_rgba("Top", tw, th, &noise(7, (tw * th * 4) as usize));
    let mut layer = top.layers.get(0).expect("a layer").clone();
    layer.rect.left += 101;
    layer.rect.right += 101;
    layer.rect.top += 250;
    layer.rect.bottom += 250;
    layer.mask = Some(LayerMask {
        rect: layer.rect,
        data: Some(noise(3, (tw * th) as usize).into()),
        ..Default::default()
    });
    doc.layers.push(layer).expect("push");
    doc.layer_compression = compression;
    doc.composite_compression = compression;
    doc
}

fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pictura-lazy-{name}-{}-{:x}",
        std::process::id(),
        u64::from_le_bytes(crate::os_random())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("doc.psd")
}

fn options(skip: bool) -> OpenOptions {
    OpenOptions {
        skip_layered_merged: skip,
        ..OpenOptions::default()
    }
}

struct Lazy {
    doc: Document,
    preview: Option<MergedPreview>,
    pin: Arc<SourcePin>,
}

fn open_lazy(path: &Path, skip: bool) -> Result<Lazy, PsdError> {
    let pin = SourcePin::open(path).expect("pin");
    let bytes = pin.map().expect("map");
    let options = OpenOptions {
        source: Some(Arc::clone(&pin)),
        ..options(skip)
    };
    let (doc, preview) = read_psd_lazy(&bytes, &options, ImageBudget::default())?;
    Ok(Lazy { doc, preview, pin })
}

fn open_eager(bytes: &[u8], skip: bool) -> Result<Document, PsdError> {
    read_psd_within_options(bytes, &options(skip), ImageBudget::default())
}

/// Every layer and mask plane of `doc`.
fn planes(doc: &Document) -> Vec<&Plane<u8>> {
    let mut out = Vec::new();
    for layer in doc.layers.iter() {
        out.extend(layer.channels.iter().map(|c| &c.data));
        if let Some(data) = layer.mask.as_ref().and_then(|m| m.data.as_ref()) {
            out.push(data);
        }
    }
    out
}

fn written(path: &Path, doc: &Document) -> Vec<u8> {
    let bytes = write_psd(doc).expect("write");
    std::fs::write(path, &bytes).unwrap();
    bytes
}

// [lp_identical]
#[test]
fn lazy_open_matches_eager() {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for entry in std::fs::read_dir(corpus).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "psd") {
            files.push((path.display().to_string(), std::fs::read(&path).unwrap()));
        }
    }
    for compression in [Compression::Rle, Compression::Raw] {
        let bytes = write_psd(&fixture(compression)).expect("write");
        files.push((format!("fixture {compression:?}"), bytes));
    }
    for (name, bytes) in files {
        let path = temp_path("identical");
        std::fs::write(&path, &bytes).unwrap();
        let eager = open_eager(&bytes, false);
        let lazy = open_lazy(&path, false).map(|l| l.doc);
        match (eager, lazy) {
            (Ok(eager), Ok(lazy)) => {
                assert!(eager == lazy, "{name}: documents differ");
                assert!(
                    write_psd(&eager).ok() == write_psd(&lazy).ok(),
                    "{name}: saved bytes differ"
                );
            }
            (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string(), "{name}"),
            (a, b) => panic!("{name}: eager {:?}, lazy {:?}", a.err(), b.err()),
        }
    }
}

// [lp_no_decode]
#[test]
fn lazy_open_decodes_no_layer_pixels() {
    let path = temp_path("no-decode");
    written(&path, &fixture(Compression::Rle));
    let lazy = open_lazy(&path, true).expect("open");
    for plane in planes(&lazy.doc) {
        assert!(plane.tile_count() > 0);
        for i in 0..plane.tile_count() {
            assert_eq!(plane.tile_residency(i), Some(Residency::Backed), "tile {i}");
        }
    }
    assert_eq!(lazy.pin.bands_decoded(), 0);
    assert_eq!(lazy.pin.bytes_read(), 0);
}

// [lp_eager_kinds]
#[test]
fn zip_and_deep_channels_open_eagerly() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = vec![
        (
            "zip".to_string(),
            write_psd(&fixture(Compression::Zip)).unwrap(),
        ),
        (
            "zip prediction".to_string(),
            write_psd(&fixture(Compression::ZipPrediction)).unwrap(),
        ),
    ];
    for deep in ["rgb16_layered.psd", "rgb32_layered.psd"] {
        files.push((deep.to_string(), std::fs::read(corpus.join(deep)).unwrap()));
    }
    for (name, bytes) in files {
        let path = temp_path("eager-kinds");
        std::fs::write(&path, &bytes).unwrap();
        let lazy = open_lazy(&path, false).expect("open");
        assert!(!lazy.doc.layers.is_empty(), "{name}");
        for plane in planes(&lazy.doc) {
            for i in 0..plane.tile_count() {
                assert_ne!(plane.tile_residency(i), Some(Residency::Backed), "{name}");
            }
        }
        assert!(lazy.doc == open_eager(&bytes, false).unwrap(), "{name}");
    }
}

// [lp_rows]
#[test]
fn a_backed_tile_reads_only_its_band() {
    let path = temp_path("rows");
    written(&path, &fixture(Compression::Raw));
    let lazy = open_lazy(&path, true).expect("open");
    let red = &lazy.doc.layers.get(0).unwrap().channels[0].data;
    let first = red.tile_at(0).expect("tile");
    assert_eq!(first.h, TILE);
    drop(first);
    assert_eq!(lazy.pin.bytes_read(), (TILE * W as usize) as u64);
    assert_eq!(lazy.pin.bands_decoded(), 1);
    for i in 1..W.div_ceil(TILE as u32) as usize {
        drop(red.tile_at(i).expect("a sibling"));
    }
    assert_eq!(
        lazy.pin.bands_decoded(),
        1,
        "the siblings read the cached band"
    );
    assert_eq!(lazy.pin.bytes_read(), (TILE * W as usize) as u64);
}

/// The fixture with its first layer channel's payload cut to `keep` bytes
/// after its compression word, the channel's declared length lowered to
/// match; and the offset of that channel's length field.
fn short_channel(compression: Compression, keep: usize) -> Vec<u8> {
    let bytes = write_psd(&fixture(compression)).unwrap();
    let eager = open_eager(&bytes, false).unwrap();
    assert!(!eager.layers.is_empty());
    // The first record's first channel: id (2) then length (4), right after
    // the record's rect (16) and channel count (2), after the layer count.
    let mut r = Reader::new(&bytes);
    r.pos = 26;
    let cm = r.u32().unwrap() as usize;
    r.pos += cm;
    let ir = r.u32().unwrap() as usize;
    r.pos += ir;
    r.pos += 4 + 4 + 2; // section length, info length, layer count
    let len_at = r.pos + 16 + 2 + 2;
    let old = u32::from_be_bytes(bytes[len_at..len_at + 4].try_into().unwrap()) as usize;
    let new = 2 + keep;
    assert!(new < old);
    // Lowering the declared length keeps every other offset in place: the
    // reader takes the channel's bytes by its declared length, so the
    // removed tail is simply the next channel's leading bytes.
    let mut out = bytes.clone();
    out[len_at..len_at + 4].copy_from_slice(&(new as u32).to_be_bytes());
    out
}

// [lp_hostile_index]
#[test]
fn lazy_index_rejects_short_channels() {
    let cases = [
        // The count table itself is cut short.
        short_channel(Compression::Rle, 100),
        // The table fits, the rows do not.
        short_channel(Compression::Rle, H as usize * 2 + 10),
        // A raw channel shorter than its rows.
        short_channel(Compression::Raw, 1000),
    ];
    for (i, bytes) in cases.into_iter().enumerate() {
        let path = temp_path("hostile-index");
        std::fs::write(&path, &bytes).unwrap();
        let eager = open_eager(&bytes, false).err();
        let lazy = open_lazy(&path, false).err();
        assert!(lazy.is_some(), "case {i}: lazy opened");
        assert!(eager.is_some(), "case {i}: eager opened");
        assert_eq!(
            std::mem::discriminant(&eager.unwrap()),
            std::mem::discriminant(&lazy.unwrap()),
            "case {i}"
        );
    }
    // A channel ending past the file.
    let bytes = write_psd(&fixture(Compression::Rle)).unwrap();
    let cut = &bytes[..bytes.len() / 2];
    let path = temp_path("hostile-end");
    std::fs::write(&path, cut).unwrap();
    let eager = open_eager(cut, false).expect_err("eager refuses");
    let lazy = open_lazy(&path, false).err().expect("lazy refuses");
    assert_eq!(
        std::mem::discriminant(&eager),
        std::mem::discriminant(&lazy)
    );
}

// [lp_hostile_decode]
#[test]
fn lazy_decode_of_a_truncated_source_is_unreadable() {
    let path = temp_path("hostile-decode");
    let bytes = written(&path, &fixture(Compression::Rle));
    let lazy = open_lazy(&path, true).expect("open");
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(bytes.len() as u64 / 3).unwrap();
    drop(file);
    let red = &lazy.doc.layers.get(0).unwrap().channels[0].data;
    let last = red.tile_count() - 1;
    let tile = red.tile_at(last).expect("tile");
    assert!((0..tile.h).all(|dy| (0..tile.w).all(|dx| tile.get(dx, dy) == 0)));
    drop(tile);
    assert_eq!(red.tile_residency(last), Some(Residency::Unreadable));
    assert!(lazy.pin.rewritten());
}

// [lp_preview]
#[test]
fn merged_preview_matches_the_merged_image() {
    for compression in [Compression::Rle, Compression::Raw] {
        let path = temp_path("preview");
        let bytes = written(&path, &fixture(compression));
        let lazy = open_lazy(&path, true).expect("open");
        let preview = lazy.preview.expect("a preview");
        assert_eq!(preview.size(), (W, H));
        assert_eq!(lazy.doc.composite.width, 0, "the merged image is skipped");
        let merged = open_eager(&bytes, false).unwrap().composite;
        let rect = PsdRect {
            top: 200,
            left: 50,
            bottom: 300,
            right: 590,
        };
        let region = preview.region(rect);
        let (rw, rh) = (rect.width() as usize, rect.height() as usize);
        let n = rw * rh;
        let got = region.data.as_slice();
        let full = merged.data.as_slice();
        let plane = (W * H) as usize;
        for c in 0..3 {
            for y in 0..rh {
                for x in 0..rw {
                    let at = (rect.top as usize + y) * W as usize + rect.left as usize + x;
                    assert_eq!(
                        got[c * n + y * rw + x],
                        full[c * plane + at],
                        "{compression:?} c{c}"
                    );
                }
            }
        }
        assert!(got[3 * n..].iter().all(|&a| a == 255));
        // A rectangle reaching outside the image is transparent there.
        let outside = preview.region(PsdRect {
            top: -2,
            left: -2,
            bottom: 2,
            right: 2,
        });
        let alpha = &outside.data.as_slice()[3 * 16..];
        assert_eq!(alpha[0], 0);
        assert_eq!(alpha[15], 255);
    }
}

// [lp_preview_gate]
#[test]
fn merged_preview_needs_a_visible_background() {
    let mut named = fixture(Compression::Rle);
    named.layers.get_mut(0).unwrap().name = "Paper".into();
    named.layers.get_mut(0).unwrap().background = false;
    let mut hidden = fixture(Compression::Rle);
    hidden.layers.get_mut(0).unwrap().visible = false;
    for (name, doc) in [("not a Background", named), ("hidden", hidden)] {
        let path = temp_path("preview-gate");
        written(&path, &doc);
        let lazy = open_lazy(&path, true).expect("open");
        assert!(lazy.preview.is_none(), "{name}");
    }
}

/// Every layer and mask sample of `doc`, in order.
fn samples(doc: &Document) -> Vec<Vec<u8>> {
    planes(doc)
        .into_iter()
        .map(|p| p.as_slice().to_vec())
        .collect()
}

// [sb_replaced]
#[test]
fn source_replaced_keeps_backed_tiles() {
    let path = temp_path("replaced");
    let bytes = written(&path, &fixture(Compression::Rle));
    let lazy = open_lazy(&path, true).expect("open");
    let other = path.with_extension("other");
    std::fs::write(&other, write_psd(&fixture(Compression::Raw)).unwrap()).unwrap();
    std::fs::rename(&other, &path).unwrap();
    assert!(!lazy.pin.rewritten());
    assert_eq!(
        samples(&lazy.doc),
        samples(&open_eager(&bytes, true).unwrap())
    );
}

// [sb_deleted]
#[test]
fn source_deleted_while_open() {
    let path = temp_path("deleted");
    let bytes = written(&path, &fixture(Compression::Rle));
    let lazy = open_lazy(&path, true).expect("open");
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        samples(&lazy.doc),
        samples(&open_eager(&bytes, true).unwrap())
    );
}

// [sb_modified]
#[test]
fn source_modified_in_place_marks_tiles_unreadable() {
    let path = temp_path("modified");
    let bytes = written(&path, &fixture(Compression::Rle));
    let lazy = open_lazy(&path, true).expect("open");
    let mut other = bytes.clone();
    other.extend_from_slice(&[0; 16]);
    std::fs::write(&path, &other).unwrap();
    let red = &lazy.doc.layers.get(0).unwrap().channels[0].data;
    drop(red.tile_at(0));
    assert_eq!(red.tile_residency(0), Some(Residency::Unreadable));
    assert!(lazy.pin.rewritten());
}
