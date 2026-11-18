//! Golden digests of the PSD and raster Save As writers (#1320).
//!
//! The digests were recorded from the whole-image writers at the commit before
//! the save was streamed; the streamed writers must reproduce them byte for
//! byte. `PICTURA_RECORD_GOLDEN=1` prints the current digests instead of
//! checking them (only for re-recording after an intended format change).

use pictura_codec::{SaveKind, SaveOptions};
use pictura_core::{Channel, Compression, Document, LayerMask};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn noise(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        })
        .collect()
}

/// A layered RGB document over several bands with a partial tile column:
/// noise, flat runs (so RLE rows differ), a masked offset layer and an extra
/// alpha channel. `transparent` gives the composite varying alpha.
fn layered(width: u32, height: u32, transparent: bool) -> Document {
    let n = (width * height) as usize;
    let mut rgba = noise(11, n * 4);
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = (i % width as usize, i / width as usize);
        if y % 7 < 3 || x < 40 {
            px[..3].copy_from_slice(&[200, 120, 40]);
        }
        px[3] = if transparent {
            ((x + y) % 256) as u8
        } else {
            255
        };
    }
    let mut doc = Document::from_rgba("Background", width, height, &rgba);
    let (tw, th) = (width / 2, height / 2);
    let mut top_rgba = noise(7, (tw * th * 4) as usize);
    for px in top_rgba.as_chunks_mut::<4>().0.iter_mut().step_by(3) {
        px.copy_from_slice(&[9, 9, 9, 255]);
    }
    let top = Document::from_rgba("Top", tw, th, &top_rgba);
    let mut layer = top.layers.get(0).expect("a layer").clone();
    layer.rect.left += 37;
    layer.rect.right += 37;
    layer.rect.top += 101;
    layer.rect.bottom += 101;
    layer.mask = Some(LayerMask {
        rect: layer.rect,
        data: Some(noise(3, (tw * th) as usize).into()),
        ..Default::default()
    });
    doc.layers.push(layer).expect("push");
    let flat = Document::from_rgba("Flat", width, height, &vec![77u8; n * 4]);
    doc.layers
        .push(flat.layers.get(0).expect("a layer").clone())
        .expect("push");
    doc.channels.push(Channel {
        id: 0,
        data: noise(5, n).into(),
    });
    doc
}

fn fixture(name: &str) -> Document {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    pictura_codec::read_psd(&std::fs::read(path).expect("fixture")).expect("fixture reads")
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

struct Golden {
    record: bool,
    expected: &'static [(&'static str, &'static str)],
    failures: Vec<String>,
}

impl Golden {
    fn new(expected: &'static [(&'static str, &'static str)]) -> Self {
        Self {
            record: std::env::var_os("PICTURA_RECORD_GOLDEN").is_some(),
            expected,
            failures: Vec::new(),
        }
    }

    fn check(&mut self, name: &str, bytes: &[u8]) {
        let digest = hex(bytes);
        if self.record {
            println!("    (\"{name}\", \"{digest}\"),");
            return;
        }
        match self.expected.iter().find(|(n, _)| *n == name) {
            Some((_, want)) if *want == digest => {}
            Some((_, want)) => self
                .failures
                .push(format!("{name}: {digest}, golden {want}")),
            None => self.failures.push(format!("{name}: no golden digest")),
        }
    }

    fn finish(self) {
        assert!(
            self.failures.is_empty(),
            "digests differ:\n{}",
            self.failures.join("\n")
        );
    }
}

const COMPRESSIONS: [(&str, Compression); 4] = [
    ("rle", Compression::Rle),
    ("raw", Compression::Raw),
    ("zip", Compression::Zip),
    ("zipp", Compression::ZipPrediction),
];

// [sps_golden]
#[test]
fn psd_encodes_match_golden_digests() {
    let mut golden = Golden::new(GOLDEN);
    let mut tiled = layered(600, 1100, false);
    for layer in tiled.layers.iter_mut() {
        let (w, h) = (layer.rect.width() as usize, layer.rect.height() as usize);
        for channel in layer.channels_mut() {
            channel.data.tile(w, h);
        }
    }
    for (name, base) in [("flat", layered(600, 1100, true)), ("tiled", tiled)] {
        for (layer_name, layer_compression) in COMPRESSIONS {
            for (comp_name, composite_compression) in COMPRESSIONS {
                let mut doc = base.clone();
                doc.layer_compression = layer_compression;
                doc.composite_compression = composite_compression;
                let psd = pictura_codec::write_psd(&doc).expect("psd");
                golden.check(&format!("{name}-{layer_name}-{comp_name}.psd"), &psd);
                if layer_compression == composite_compression {
                    let psb = pictura_codec::write_psb(&doc).expect("psb");
                    golden.check(&format!("{name}-{layer_name}.psb"), &psb);
                }
            }
        }
    }
    for name in [
        "gray.psd",
        "cmyk.psd",
        "lab.psd",
        "indexed.psd",
        "bitmap.psd",
        "rgb16_layered.psd",
        "rgb16_tagged.psd",
        "rgb32_layered.psd",
        "masked.psd",
    ] {
        let doc = fixture(name);
        golden.check(name, &pictura_codec::write_psd(&doc).expect("psd"));
    }
    golden.finish();
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("pictura-save-golden-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Every file `save_document` left in a fresh directory, in name order.
fn saved(dir: &Path, kind: SaveKind, doc: &Document, options: &SaveOptions, name: &str) -> Vec<u8> {
    let sub = dir.join(name);
    std::fs::create_dir_all(&sub).expect("case dir");
    let jpeg = |samples: &[u8], w: u32, h: u32, c: u8, q: u8| {
        let mut out = format!("jpeg {w}x{h}x{c} q{q} ").into_bytes();
        out.extend_from_slice(hex(samples).as_bytes());
        Some(out)
    };
    let result = pictura_codec::save_document_with_jpeg(
        kind,
        doc,
        options,
        &sub.join(format!("out.{}", extension(kind))),
        Some(&jpeg),
    );
    let mut files: Vec<PathBuf> = std::fs::read_dir(&sub)
        .expect("case dir")
        .map(|e| e.expect("entry").path())
        .collect();
    files.sort();
    let mut all = match result {
        Ok(()) => Vec::new(),
        Err(e) => format!("error {e}").into_bytes(),
    };
    for file in files {
        all.extend_from_slice(file.file_name().unwrap().to_string_lossy().as_bytes());
        all.extend_from_slice(&std::fs::read(&file).expect("saved file"));
    }
    all
}

fn extension(kind: SaveKind) -> &'static str {
    match kind {
        SaveKind::Png => "png",
        SaveKind::Bmp => "bmp",
        SaveKind::Tiff => "tif",
        SaveKind::Gif => "gif",
        SaveKind::Eps | SaveKind::Dcs => "eps",
        SaveKind::Raw => "raw",
        SaveKind::Tga => "tga",
        SaveKind::Cineon => "cin",
        SaveKind::Jpeg => "jpg",
    }
}

fn opts(f: impl FnOnce(&mut SaveOptions)) -> SaveOptions {
    let mut o = SaveOptions::default();
    f(&mut o);
    o
}

fn raster_docs() -> Vec<(&'static str, Document)> {
    vec![
        ("rgba", layered(300, 530, true)),
        ("rgb", layered(300, 530, false)),
        ("gray", fixture("gray.psd")),
        ("indexed", fixture("indexed.psd")),
    ]
}

fn streaming_cases() -> Vec<(String, SaveKind, SaveOptions)> {
    let mut cases = Vec::new();
    for level in [0u8, 6, 9] {
        for interlace in [false, true] {
            cases.push((
                format!("png-{level}-{interlace}"),
                SaveKind::Png,
                opts(|o| {
                    o.png_compression = level;
                    o.png_interlace = interlace;
                }),
            ));
        }
    }
    for depth in [16u8, 24, 32] {
        for flip in [false, true] {
            cases.push((
                format!("bmp-{depth}-{flip}"),
                SaveKind::Bmp,
                opts(|o| {
                    o.bmp_depth = depth;
                    o.bmp_flip = flip;
                }),
            ));
        }
    }
    for advanced in [1u8, 2] {
        cases.push((
            format!("bmp-16-adv{advanced}"),
            SaveKind::Bmp,
            opts(|o| {
                o.bmp_depth = 16;
                o.bmp_advanced = advanced;
            }),
        ));
    }
    cases.push((
        "bmp-24-os2".into(),
        SaveKind::Bmp,
        opts(|o| {
            o.bmp_depth = 24;
            o.bmp_os2 = true;
        }),
    ));
    for depth in [16u8, 24, 32] {
        for rle in [false, true] {
            cases.push((
                format!("tga-{depth}-{rle}"),
                SaveKind::Tga,
                opts(|o| {
                    o.tga_depth = depth;
                    o.tga_rle = rle;
                }),
            ));
        }
    }
    for (header, interleaved) in [(0u32, true), (0, false), (37, true), (64, false)] {
        cases.push((
            format!("raw-{header}-{interleaved}"),
            SaveKind::Raw,
            opts(|o| {
                o.raw_header = header;
                o.raw_interleaved = interleaved;
            }),
        ));
    }
    cases.push(("cineon".into(), SaveKind::Cineon, opts(|_| {})));
    for compression in [0u8, 1, 2, 3] {
        for planar in [false, true] {
            for mac in [false, true] {
                cases.push((
                    format!("tiff-c{compression}-p{planar}-m{mac}"),
                    SaveKind::Tiff,
                    opts(|o| {
                        o.tiff_compression = compression;
                        o.tiff_pixel_order = planar;
                        o.tiff_byte_order_mac = mac;
                        o.tiff_layer_compression = 2;
                    }),
                ));
            }
        }
    }
    for layers in [0u8, 1] {
        for pyramid in [false, true] {
            for transparency in [false, true] {
                cases.push((
                    format!("tiff-l{layers}-y{pyramid}-t{transparency}"),
                    SaveKind::Tiff,
                    opts(|o| {
                        o.tiff_compression = 2;
                        o.tiff_layer_compression = layers;
                        o.tiff_pyramid = pyramid;
                        o.tiff_transparency = transparency;
                        o.icc = Some(vec![1, 2, 3, 4, 5]);
                    }),
                ));
            }
        }
    }
    cases
}

// [srs_golden]
#[test]
fn raster_saves_match_golden_digests() {
    let mut golden = Golden::new(GOLDEN);
    let dir = temp_dir("raster");
    for (doc_name, doc) in raster_docs() {
        for (case, kind, options) in streaming_cases() {
            let name = format!("{doc_name}-{case}");
            let bytes = saved(&dir, kind, &doc, &options, &name);
            golden.check(&name, &bytes);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    golden.finish();
}

// [srs_whole_golden]
#[test]
fn whole_image_saves_match_golden_digests() {
    let mut golden = Golden::new(GOLDEN);
    let dir = temp_dir("whole");
    let mut docs = raster_docs();
    docs.push(("cmyk", fixture("cmyk.psd")));
    docs.push(("lab", fixture("lab.psd")));
    let mut cases: Vec<(String, SaveKind, SaveOptions)> = vec![
        ("gif".into(), SaveKind::Gif, opts(|_| {})),
        (
            "gif-i".into(),
            SaveKind::Gif,
            opts(|o| o.gif_interlaced = true),
        ),
        ("eps".into(), SaveKind::Eps, opts(|_| {})),
        ("dcs".into(), SaveKind::Dcs, opts(|_| {})),
        (
            "tiff-jpeg".into(),
            SaveKind::Tiff,
            opts(|o| {
                o.tiff_compression = 4;
                o.tiff_transparency = false;
                o.tiff_layer_compression = 2;
            }),
        ),
    ];
    for depth in [1u8, 4, 8] {
        cases.push((
            format!("bmp-{depth}"),
            SaveKind::Bmp,
            opts(|o| o.bmp_depth = depth),
        ));
    }
    cases.push((
        "bmp-8-rle".into(),
        SaveKind::Bmp,
        opts(|o| {
            o.bmp_depth = 8;
            o.bmp_rle = true;
        }),
    ));
    for (doc_name, doc) in &docs {
        let mode_cases: Vec<(String, SaveKind, SaveOptions)> =
            if matches!(*doc_name, "cmyk" | "lab") {
                vec![
                    ("tiff".into(), SaveKind::Tiff, opts(|_| {})),
                    ("raw".into(), SaveKind::Raw, opts(|_| {})),
                    ("png".into(), SaveKind::Png, opts(|_| {})),
                ]
            } else {
                cases.clone()
            };
        for (case, kind, options) in mode_cases {
            let name = format!("{doc_name}-{case}");
            let bytes = saved(&dir, kind, doc, &options, &name);
            golden.check(&name, &bytes);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    golden.finish();
}

#[rustfmt::skip]
const GOLDEN: &[(&str, &str)] = &[
    ("bitmap.psd", "1183f5fae15e325c5d2e228a98bf040db0dc3714d02852b3a29b066db1d23930"),
    ("cmyk-png", "36c274d7f0d68b37d22f4d4902b35a3123192c1ae8d7b53d744425f3fbc9a3f0"),
    ("cmyk-raw", "6ebe9df20a5223b6926ecef3f3c4a9de1d4b1bc9cf4bd4c427b066499df92817"),
    ("cmyk-tiff", "4ec1acabcc0a9c4f5c336532254df28f9e271b65cbd460f44f6a2135c18e3440"),
    ("cmyk.psd", "1dcc7d3b575d76d2b348c1bc94b6999bde10b678d2084c2202e46ae0967c77bd"),
    ("flat-raw-raw.psd", "a3348348d886a5df2fa2085e6e2e020ba965ea7d41d268fc066fac9c62ea2e10"),
    ("flat-raw-rle.psd", "00643840c4480828b50a3064740f58851c50eabe345bff8fce44d309ce7fcac6"),
    ("flat-raw-zip.psd", "ce3bad2d4f53097cd8446379935797ac59501909bcaec6bc37675924a0a2e444"),
    ("flat-raw-zipp.psd", "f5949d9db46f58a13937b182db7c77167461d2b46dce613abf779e565cea3e79"),
    ("flat-raw.psb", "571de6267d2838711ddfd4128ae10bb0158f71f43e845a1b84cad91294cc4d03"),
    ("flat-rle-raw.psd", "6bc570a7d11b74c34951007a372563c4e4ee140016a2e3cb7a86cdc8a1e59d6f"),
    ("flat-rle-rle.psd", "1a7f37dc304db05d45eca786a8d562f75e6cfab8042753c7a659fd4d8bb81e0f"),
    ("flat-rle-zip.psd", "e1284a3b0e0cd423bbeba37d260811b16b004a57c79f54404c5016231054e5ff"),
    ("flat-rle-zipp.psd", "a78dd3c5003e51fb43283b65a85839a628cb4b8b8a919660328566fba417a73b"),
    ("flat-rle.psb", "ece032b74987583043d2ed25639005652ab6003b4b95b613709db46fdaca5324"),
    ("flat-zip-raw.psd", "d4a84a8946d47b5b0c895d049b55aff865b1d67711b22567109966f0cd3ddc27"),
    ("flat-zip-rle.psd", "e33a01d5a0ffe4121ef31a6620f1b6da9e2d9b51b50de584e6590e866010270e"),
    ("flat-zip-zip.psd", "b7134bc62bdf8cd3c64c001cbcff29cd6e30b4d376487c247335237cfbab6d3d"),
    ("flat-zip-zipp.psd", "9dff6b0dde63c85122eebfdb48366a58a45e91ae0e845349374ee15f1f3e77b5"),
    ("flat-zip.psb", "35ccc49fcd19ca99cb2d47aaee6976f57f73bea86b1e1cae1151d6574d11abbd"),
    ("flat-zipp-raw.psd", "1d52b337b77ebcf52e8b9070b9b2b11681d397b758e527865d9bc2c57e3f396d"),
    ("flat-zipp-rle.psd", "84edcefca6b075565e88755cf648cd6c6c1cc42a227031c28a62701c8ebf8462"),
    ("flat-zipp-zip.psd", "4189f8391bebc82a3ff37001d601c704ee55caeaf98236a1f10280547a690474"),
    ("flat-zipp-zipp.psd", "83c297029818117cf39f053712eea30257053469440762f27bc167a1eda3eb67"),
    ("flat-zipp.psb", "ec73fc930d984143cd95028686fbb6ba93aa15037ddb1a10e07ee4ad412cc033"),
    ("gray-bmp-1", "b31853e2c0fafd877bb7efccc7bf7203ed4946eefa42fae858d61cc9766451cb"),
    ("gray-bmp-16-adv1", "a913eb2f75b8e40e174d90556ca641e01e86fbf0015d88d9d835096257d90137"),
    ("gray-bmp-16-adv2", "bbe5e1bd3313f358d05a34c44a29b5549d6e325c855976d009fe280c96bd4bf9"),
    ("gray-bmp-16-false", "79f3374f4dc39e7a121244a76ec58ab52cf77fcbb3986e06ba30b317201c0b28"),
    ("gray-bmp-16-true", "1d61927218dd96bde7d8bd2eb3e38a677b9271687d414325f4ae3adc1b9c2ac6"),
    ("gray-bmp-24-false", "ebfc35e351c52a627e575a4de1b33f6f117346ad46da5356e071315479b3078b"),
    ("gray-bmp-24-os2", "a62ef2fb3c162a824993ef90f6c76b0c60db7f18e88e86c70a5cfa629e74fcaa"),
    ("gray-bmp-24-true", "2e5f4aeb33041fb20fa7533aedb53a41a9c192c38e41396c34cf3c43cdce4b9d"),
    ("gray-bmp-32-false", "ae03656fa6624be3f1b8e67753c8c6502a2eedf6d0402378e094e1c2ba042eb3"),
    ("gray-bmp-32-true", "18a45aef1a95f8e7f8bb30ae7da8c810a7de8b25efc53e6e8636b6db1a9d483e"),
    ("gray-bmp-4", "48d393e6c4e706aa3b2123de12dcf4fc246b0752ff2fe9f3770e8d96f54d00fe"),
    ("gray-bmp-8", "a15f0e805a22113f7853942a4f3ba6e952d2f4ad512429a1d7b4844bf1ab370f"),
    ("gray-bmp-8-rle", "adb4456ddcd00db5d797798915e639d939cb74e5c4819efab2e888e9cf6ec4cd"),
    ("gray-cineon", "1c6fae428879435d47819010eeb6b7419310594d136220349b8a573239189cc3"),
    ("gray-dcs", "d49172394edbd8b2417d9b25a50f872697e3f493b76859d9042663b50600a682"),
    ("gray-eps", "ded29a0babac74e11c068c657e422b6b811e23e6c95145b4c5051733d50258d0"),
    ("gray-gif", "6cebc330490bceba52f43feef2682fcea5974ef530334cc42f6c3e8ef86cf70f"),
    ("gray-gif-i", "8993aa4e2ac72f0e0fb265d264d9161d0d83221e1f21576b43f9058c0c585bc3"),
    ("gray-png-0-false", "61cf936f2744b92d03bb84924446820bc19e234a69191b249d52bebf9b37e4c2"),
    ("gray-png-0-true", "b48105b51b547a1a487c0bc22382215e7be515b0784f0915c9160287e8e22ee0"),
    ("gray-png-6-false", "49f05af5f2f6015872ce6291916e361b44fd4d775e1f768f07284b3dd543f914"),
    ("gray-png-6-true", "13a7d175ace0d1a5992c17842402127cadc405a4e2feb1e6bcd32b063b3e98fa"),
    ("gray-png-9-false", "3683c52caa65f1729a043ce2d758fa882521870ef2e3322621b5919f5bbd8b7d"),
    ("gray-png-9-true", "ace2a60b8020b8235de68ada9a93cb676be5cda24ac5de81f4d1a8fd01f3faa6"),
    ("gray-raw-0-false", "8f43efcd32b5eabd8c945ee9d0442cc097cbed471dc66c765661b7f9adcca692"),
    ("gray-raw-0-true", "8f43efcd32b5eabd8c945ee9d0442cc097cbed471dc66c765661b7f9adcca692"),
    ("gray-raw-37-true", "b34643b45818ea94cb976ee36b40e2f1e79076836322a227c815dcaf47755771"),
    ("gray-raw-64-false", "75fafeaa0466550d0c6bd172f3a1a0a37878751b665af471eb499930b977a03b"),
    ("gray-tga-16-false", "bcd66fe93bc0915c00bf12bd4577f36a3a6b6460bfd7917383482642c88eecd5"),
    ("gray-tga-16-true", "c96cbc48a04d305ac8fd91cd56ec5c5f0c85a300d02eceea46e69297ab20b2ad"),
    ("gray-tga-24-false", "e77407d128b301a0511182603a85e77ce4571f3fd3b0776fe02210d316e923a4"),
    ("gray-tga-24-true", "4ea84baf571f08bfd8a64088bbdabd2acf867e2dfd80c4757bdfea8dd48b0a3f"),
    ("gray-tga-32-false", "78e164d2928d8601d0592aaae351d2992b083e1f9d3d23fab121f36620c8fdb8"),
    ("gray-tga-32-true", "8599f97127c95043a2a09225e95fe45ac7c29660567a55d6f3405212a5d13899"),
    ("gray-tiff-c0-pfalse-mfalse", "fda00b1a07e10e4cc05dfc579b12948f06439781d52fb108c118bead3c696826"),
    ("gray-tiff-c0-pfalse-mtrue", "dc9f8c6284bb323b309c927cfcc1e74c31e8580397a328bf3b4017b1c4d30602"),
    ("gray-tiff-c0-ptrue-mfalse", "64e74e1485bf5d41ac03ca5e1d69117b71ede0f7981ed5fd8be86fbab005dea0"),
    ("gray-tiff-c0-ptrue-mtrue", "2afec51eccfacf73f0c9f0626d9552ee39aaf42004c169092c751987bbe79051"),
    ("gray-tiff-c1-pfalse-mfalse", "1427e64432c9809d3f9c03288121d0c2e8bccbc67212dd0dc5765ee6fd7d4e76"),
    ("gray-tiff-c1-pfalse-mtrue", "1b113109125fb06cf42ef19c993d7e2416b28b8cdea1f4d86c0b76a19332c8c0"),
    ("gray-tiff-c1-ptrue-mfalse", "e61ac4c9a88415bd465316251e11d988731cbe894f182470937d6b19f992e26d"),
    ("gray-tiff-c1-ptrue-mtrue", "50daa7883bfdbb315096507457a628cc27097b2c45304414b288747c98b61729"),
    ("gray-tiff-c2-pfalse-mfalse", "b623450e8a8f46efdadc0ea33ff60181bacb892de81a8b2807b48071e73206aa"),
    ("gray-tiff-c2-pfalse-mtrue", "de42c53e45360e313386f049c1f04817aa82cc38150fb2e014a3e5bbcf3b883a"),
    ("gray-tiff-c2-ptrue-mfalse", "8604fb83568c900b9c69dba6525c398edc76794b3dc9e6698efbacc3115f03b7"),
    ("gray-tiff-c2-ptrue-mtrue", "2f59a4c7c385763fbe59e8d7cb4f9e7ca6746325083cdb0ea037cccd41233b88"),
    ("gray-tiff-c3-pfalse-mfalse", "d425eaeb98a1fdeddc2501a60e64ebf170a5cddeaae849999ea90f0c92a4cb24"),
    ("gray-tiff-c3-pfalse-mtrue", "770f82804d49b7ac479866732cd80291db81fa63dbe0d83f8ad755876c95aebf"),
    ("gray-tiff-c3-ptrue-mfalse", "ff234d4e9dc4d4d911de1c014e2d03df05fab9b345383e2b94070c94b645079d"),
    ("gray-tiff-c3-ptrue-mtrue", "96af90704c07493624b2498cb3d1f0d28cef94f19fa2fc64321771fc48e3c112"),
    ("gray-tiff-jpeg", "3c15e636114df96fb6099fda22eca8a2370bd79cc61a2e7251ff9dc045a450e4"),
    ("gray-tiff-l0-yfalse-tfalse", "daab214a84d85cb1e3e9849353b82084f71f57d758764e2dfe86b2416462df52"),
    ("gray-tiff-l0-yfalse-ttrue", "daab214a84d85cb1e3e9849353b82084f71f57d758764e2dfe86b2416462df52"),
    ("gray-tiff-l0-ytrue-tfalse", "fdf59d13c4fb4e255788738f693cbcb51b5cee47daab4425ae63282f9812fbb5"),
    ("gray-tiff-l0-ytrue-ttrue", "fdf59d13c4fb4e255788738f693cbcb51b5cee47daab4425ae63282f9812fbb5"),
    ("gray-tiff-l1-yfalse-tfalse", "03d2151783dec02f29fbc9b53c5cf36e7a2e7c7389654621293ac9874c0e3ac6"),
    ("gray-tiff-l1-yfalse-ttrue", "03d2151783dec02f29fbc9b53c5cf36e7a2e7c7389654621293ac9874c0e3ac6"),
    ("gray-tiff-l1-ytrue-tfalse", "7c6fdaf50baffdc55a0e714fb6fa23703177582998650dadba693ae7bc24c4f9"),
    ("gray-tiff-l1-ytrue-ttrue", "7c6fdaf50baffdc55a0e714fb6fa23703177582998650dadba693ae7bc24c4f9"),
    ("gray.psd", "d5bad8416102e33b44f56957b45d646ec90c5a815e3cb946d7626c28ffbfa7cc"),
    ("indexed-bmp-1", "920782206eb41fbb840b9187e36c48bc139aa8714b58878e926f1bcfab3c01de"),
    ("indexed-bmp-16-adv1", "9e72d0b0439273fb73c53e11ebd835c511a7b70c9276edb4c508aeada9841a6a"),
    ("indexed-bmp-16-adv2", "601fef29d52922abeb73a8c3959d77a34927a553eaec8a94c039979700531679"),
    ("indexed-bmp-16-false", "bec9644bb2d06123a05afbfac39d1a5eabd4181f4c5aa5ac96007a40e9403b91"),
    ("indexed-bmp-16-true", "11d8c6c7ee2a1dad96e89c4bebebd812ad97d0a671626d7d4e757906315d6b07"),
    ("indexed-bmp-24-false", "dedf1af6b091bf021df89fe1cfce85f503a25f17d0f3a5a8f5d1b439a5bcc6ef"),
    ("indexed-bmp-24-os2", "965251d6a481abc0989ebff5e132f15817660e85cc297f5451c64aed360f0897"),
    ("indexed-bmp-24-true", "cdfbc157bc8e6471ef97df241a0b356d6cdfaaa4fbce1f383642c263d5d55459"),
    ("indexed-bmp-32-false", "e3eb8f944e1e3d3706a318fcf216d7da194377e5ce9f4ded1797a387f7b16a1c"),
    ("indexed-bmp-32-true", "f90cc25f2f1b44cf1a50d5069013bb65acffd65591e83ad38461bbae8ec2749d"),
    ("indexed-bmp-4", "fac8cc82d13dc1c2e877cb5b9adfee9d67d8689f6655777efaa1ec35f2b41215"),
    ("indexed-bmp-8", "b51d560683d347169656b59d7feafbd89ceb77ae1ef405a63db6b5c08c17a696"),
    ("indexed-bmp-8-rle", "425f9bd799fd51dc2f408760fd57d022efce4555e76cfc15efc55dc3d61d11cf"),
    ("indexed-cineon", "c9af6e079697d1480ffe52133fdcbeed7f26d0d8682d3a870fad01e7958835af"),
    ("indexed-dcs", "ef2d25c6f893abd4a2380ba87a59b7b508eef9dcf2303373f4eb5f1377a5567d"),
    ("indexed-eps", "1322d9f4150f8a6990a083b5edb2012f4334d572c307763efb6d6da19b65bb00"),
    ("indexed-gif", "7f20ff405ac0adaf75db727fec9be13f63e0e5daf237331688b9628c29ade6c8"),
    ("indexed-gif-i", "9c4c70e67230fdc2118ef1538db8bba3836b63f83e7ec7ae20989bb956a15fc6"),
    ("indexed-png-0-false", "a5a4202d7b5950ca9d1164c623e0577cdf22af7fb2ab9dd8d435fb8a9eae0f1a"),
    ("indexed-png-0-true", "5c0a7d271e3b1eb0438348e74da23378d93b877635c0f1d490b32ae7c9f7add1"),
    ("indexed-png-6-false", "8585a192477fdbadb1d5af0b0609d8713f2b1c0f113f4db2d81b68bbfa93fa05"),
    ("indexed-png-6-true", "19eeb62a2b37bdf90115cf8a62b2ed206b7ecefd896cff6d257cef16f62b4572"),
    ("indexed-png-9-false", "ceddb46c7af73a20d193f6360dbccc818a28f0641986cef5b6645d1894e4adad"),
    ("indexed-png-9-true", "cbec3458fc41121022b06db9b5b02a8f9d177c7939a8dd709f7f60a42d224a75"),
    ("indexed-raw-0-false", "c2dad371748099bde7794b1cb1921aabc3b5f33156617ad2d7a39a374030a0e0"),
    ("indexed-raw-0-true", "9d96e72de6a125b34cb5e25b21920ada3ce0ac27914b14c231e64003ec6ebfd9"),
    ("indexed-raw-37-true", "f96db74dd5a9fb3bfc2b6d3f39717541b8f45852ab36c7e1287789cd99f6710c"),
    ("indexed-raw-64-false", "b1327e81ccfb227f4e7ef67d73e5037d9fa71aff87419e7be794d63862fd1c5a"),
    ("indexed-tga-16-false", "db5f012c15763c6e8cafbfbbd187faf44269e45b0ae837c8af0b9447b4ea2914"),
    ("indexed-tga-16-true", "7586675e475f0dd9eb56f255dfb55abfc65139487997ad73ddeeffd70300e854"),
    ("indexed-tga-24-false", "9a4edf5035ab2f38fcee48b4140b38d70d764087b6f8597cae95b88fe3566d2b"),
    ("indexed-tga-24-true", "2d4928215757d21bd015e63290f28450d102a46cc4398ab41936eec042326698"),
    ("indexed-tga-32-false", "70ef1d86ccd88e92876b6566028316b5ffe67cfc9cfe662d6241055b64265541"),
    ("indexed-tga-32-true", "26ea381c3217b9854bf108ec9c216e7c59351ab815bac356dcbe948f518c5caf"),
    ("indexed-tiff-c0-pfalse-mfalse", "b57db40574e3c6f53f0940b0800357799fccaf9fad26bcac6e72a1231f422f0c"),
    ("indexed-tiff-c0-pfalse-mtrue", "e5ac93387eb0836984b75dd647f860b9a8717657bb623f9f6f59c430267d152a"),
    ("indexed-tiff-c0-ptrue-mfalse", "d6f70663aae55a49c9156bff62843b8a82079e70500b751e875dfdf996cde146"),
    ("indexed-tiff-c0-ptrue-mtrue", "4bbbf68d51ffb2b399c653cf2905b73efbe4066034c58058715cd97ed66d38d8"),
    ("indexed-tiff-c1-pfalse-mfalse", "f9aa1b9472a5707f4917dd2415a2b91cfd2413051584effd9f31966f3375c79c"),
    ("indexed-tiff-c1-pfalse-mtrue", "8f1fefb542c4d86b9d67cc951a7c1a8bfedbdae72f4c48d51259306186efc17f"),
    ("indexed-tiff-c1-ptrue-mfalse", "3d8eaf6c4571efb46a009687994a9617724937dcb817adaeafbd26be094858cb"),
    ("indexed-tiff-c1-ptrue-mtrue", "a9ca06ede540271511b6c406e0e4bcd320bd4fcb0a663cab576a051911536324"),
    ("indexed-tiff-c2-pfalse-mfalse", "35af69b300e15c5aff250700ec59aa8ed3836adeb54f4397ac56387a614cecc0"),
    ("indexed-tiff-c2-pfalse-mtrue", "f301bff8459715916a7570545df6b8a683f1dcd6bc98ae0244bf08571dc8648c"),
    ("indexed-tiff-c2-ptrue-mfalse", "4d2a0047f4a7eab21a9ce20e298a26394a8f83c5f3da40a94eec70e8cf1349be"),
    ("indexed-tiff-c2-ptrue-mtrue", "9351031b22dfa65c44cc436877d94340055308b44af2ef79209f39c92498f524"),
    ("indexed-tiff-c3-pfalse-mfalse", "1bd90b02b4b47275c53cceb3c623ae58e25f669daaea6ee80ae9193041993d79"),
    ("indexed-tiff-c3-pfalse-mtrue", "280bf514fd7bf8380a7466361ecc7d1bdf33fe50ca891c73216eeff687dfffe2"),
    ("indexed-tiff-c3-ptrue-mfalse", "e5ac765202b1b72fd745eeaa73b78ea3c38cbd1879566c2b38f22de0ef0dc53d"),
    ("indexed-tiff-c3-ptrue-mtrue", "437b9b1031a4ae7899b4cdd0867188a095d1d48911f5df9ef6cd44c9b6ad8f88"),
    ("indexed-tiff-jpeg", "f3c029030138eba7954e455dc991c72d352f2abc9aec9e634ac4598701b502c2"),
    ("indexed-tiff-l0-yfalse-tfalse", "99c834dbb75d60ea8d189bf11b0f41ef083b97b5762a31db62c2f526b95fa195"),
    ("indexed-tiff-l0-yfalse-ttrue", "99c834dbb75d60ea8d189bf11b0f41ef083b97b5762a31db62c2f526b95fa195"),
    ("indexed-tiff-l0-ytrue-tfalse", "62fb5eeef2feaec00dc2db9c8c1d440aa8fd47197b570d79273eba432ce62a35"),
    ("indexed-tiff-l0-ytrue-ttrue", "62fb5eeef2feaec00dc2db9c8c1d440aa8fd47197b570d79273eba432ce62a35"),
    ("indexed-tiff-l1-yfalse-tfalse", "99c834dbb75d60ea8d189bf11b0f41ef083b97b5762a31db62c2f526b95fa195"),
    ("indexed-tiff-l1-yfalse-ttrue", "99c834dbb75d60ea8d189bf11b0f41ef083b97b5762a31db62c2f526b95fa195"),
    ("indexed-tiff-l1-ytrue-tfalse", "62fb5eeef2feaec00dc2db9c8c1d440aa8fd47197b570d79273eba432ce62a35"),
    ("indexed-tiff-l1-ytrue-ttrue", "62fb5eeef2feaec00dc2db9c8c1d440aa8fd47197b570d79273eba432ce62a35"),
    ("indexed.psd", "9aaa5c6c77801ef609b8773ec6cf4a73e3f977f3408bd11cfb3d3bc0e7f5b043"),
    ("lab-png", "162b8751b0bd758602da16cb6548b42b4b6a191770b1ca2e0ee7f8bc30d86e26"),
    ("lab-raw", "f31dd6be871ee357052b75d418c5119e272d531fc5608d2e8d573b124ce75645"),
    ("lab-tiff", "6af6ae9a4aa6db5e597d627c630128d721d4bf2ca5891b77df5c3507fcf98f98"),
    ("lab.psd", "cf34bcb43674231729860839767260aeac5c17f9542a6477694a929809709663"),
    ("masked.psd", "ef7b87556fe31c5223d857ddedb63c919fceb862e00116f1019c968fa501a0dc"),
    ("rgb-bmp-1", "7c0a7426f418b58a0e4a70b949d1a3126f9cb1742eb394ac764b5456bc36f297"),
    ("rgb-bmp-16-adv1", "b28318c76e665c76f4c0b573e3ab9a13f5e471d0a0aa85fa541e9770b3465f24"),
    ("rgb-bmp-16-adv2", "2f8d95067c5802bcb614999a77dbef385cbac16b200b427dc3fb7c56649e9893"),
    ("rgb-bmp-16-false", "aeab56b9d04aa9a317be76239fb6b274c8e532ad018825f316e0b0e17e6b65f2"),
    ("rgb-bmp-16-true", "c70bc141d870517cde8386e22fd6a570c34d420056bdbe89dba970a660ae4f5a"),
    ("rgb-bmp-24-false", "73d9bde3ad435f6dce5806e74b836a41e040b383fb6a7f69f52e668ad06c72a7"),
    ("rgb-bmp-24-os2", "3e31cb4c9b25c3cc459a5be78669cfb92d9a2f49b74f0e883e2705fb350cec21"),
    ("rgb-bmp-24-true", "8ced48cc2418ce898b54eec5647e8a7e7ffc705dd6fd889c79b70024182c9beb"),
    ("rgb-bmp-32-false", "c4436800e24f63f7ed22ee42348895e9c84b652136df2f520bf4a26c11ffc261"),
    ("rgb-bmp-32-true", "9aa7567bb6df0db80f383dddcc991e9677824f70845989a9c6e337ea923968ed"),
    ("rgb-bmp-4", "d16ff44022ae9314e5fe83a64dd0eff7bbe2b3585ded3557f7a380e160104214"),
    ("rgb-bmp-8", "2244826f38e45e4e7a63e49306ca651ded8eba2ae4e478c51bdc1a8c3cbf42e7"),
    ("rgb-bmp-8-rle", "c6fa51209e3384d6c419f587537e226b7cc88058562fe560a5be2ce426ddabd8"),
    ("rgb-cineon", "d407bd3c54f32c8a57d40cf1a44c53b6825a985f83355692d0a061d9da07fe37"),
    ("rgb-dcs", "cb87d6370eab99474e027652a4ceed343e035ac6f44b6096055f2b016e9e6abd"),
    ("rgb-eps", "0cba8299ecfeceacece65148050d7cc1ab5fb11e5b28237fe3fd6b1d4c2c93af"),
    ("rgb-gif", "cf51d081e4223577950144748a70e65ea57d208594c645b8a48b37bc19d1064c"),
    ("rgb-gif-i", "e38fd994fa41f519c75f245a07bf382e00f327e3d18a91c80a7765922207dd8b"),
    ("rgb-png-0-false", "5eb20599954c625d8aac3fb5236135a1537953489c14ccfe3f7a0a1645070638"),
    ("rgb-png-0-true", "a767f77d1b4599aacc3f56d96b0162cb15e897cdbf34c2f01756879565416618"),
    ("rgb-png-6-false", "104ba2218ac5b30c5f05e1bf00981b1d60ecd99a4b50f4c1d0b00577f4f62c9f"),
    ("rgb-png-6-true", "def8d5874b2b08288cf9b3537d44ea379e4cf43c109453f152965502e713bbc4"),
    ("rgb-png-9-false", "39a8c9510cba8e8db237331bc8eeb666197c5f2f3f48ac4f9af25a8302a673fb"),
    ("rgb-png-9-true", "74168e4ff71f331ec67b3399498db3cfcadc624cbb77cc66774bf52926ce1196"),
    ("rgb-raw-0-false", "1494d4d96cac6cf05d25bbf59a72a421b239aabe03df77f1e285acd61ee2ebaf"),
    ("rgb-raw-0-true", "076a6a8be5fabb838bf4fb02cf745bec7bd883fc382fb694fdeb53f74c86ae26"),
    ("rgb-raw-37-true", "64f68b820a41b79832203fb2368a579b2d5ce2a7ecbc79de52132804ba312543"),
    ("rgb-raw-64-false", "2dc6821638062bc1cea01d3a0d9fe5298ae3b1895135395272951d5624ed8d2f"),
    ("rgb-tga-16-false", "6d920e2de9d0ea7c7a61bb72df7fffe65b1d1e99e8d998e0ea4376736f6f2957"),
    ("rgb-tga-16-true", "3fb183f4ed7f50c2b317aedcba9a400ca548b4f6da9fd958b1d64a1e5966b93e"),
    ("rgb-tga-24-false", "7f0cc102fdbffe0b00c0a9e3a3952d2a9069d6be69a6010208e3df177c8b2bbb"),
    ("rgb-tga-24-true", "e225cc84fce8fd8bd7bffed357643da7ac4d4d1a36324c16f6a4723a2b529e5a"),
    ("rgb-tga-32-false", "75efebc467352c0dae60feac1ae886cfff52b7dd562aa61e4845f4fd4cb249ed"),
    ("rgb-tga-32-true", "7fc2b182e4799850416976ae7094607bcea4a39571f215f7837352a0a0fc6cfa"),
    ("rgb-tiff-c0-pfalse-mfalse", "c489dbe4aaa7e84382af25629a94186f233a634d2b3e1cdc0c5c1dcf4b7829d5"),
    ("rgb-tiff-c0-pfalse-mtrue", "d634c423fdc71b284c5ee47b5239ed756de626a9a0a894a70aaed4d7552eedde"),
    ("rgb-tiff-c0-ptrue-mfalse", "4efe06f30e50e70454bcefc783552dde4ea8d3872736267f00e346733b1b01d0"),
    ("rgb-tiff-c0-ptrue-mtrue", "edf0f6dae178a3360dd830ce80349a07b2c235e8ce528619974526d4fed92bda"),
    ("rgb-tiff-c1-pfalse-mfalse", "31703a3dcb1836fb4ec1fd1f758f7db48ad046f00e14c54d6857f20d5789ec65"),
    ("rgb-tiff-c1-pfalse-mtrue", "4cee417c27f2072c84690098b45929c89fa49446bef0e5e44b0d40c056547462"),
    ("rgb-tiff-c1-ptrue-mfalse", "efe5b13ea1243bdc9da42fff7fb31a5bdbe8c3b4298b81cff853db9da900d2dc"),
    ("rgb-tiff-c1-ptrue-mtrue", "fa5f36a519bdcc13c653dc14b86b60ba2b7fdb4f1b4665317db2817c4eea38ca"),
    ("rgb-tiff-c2-pfalse-mfalse", "617d5b23331f5b84f61d7f8c9cfa3b3a57fe883220a2b62075b9687d15beea52"),
    ("rgb-tiff-c2-pfalse-mtrue", "29d1c5757c5a7a645cb6499f329c14a6a240d59f3315b67dff2adc9eb6526193"),
    ("rgb-tiff-c2-ptrue-mfalse", "af9e1f8a043cbb85a29c376a3dc6016fa2b0562fa2d189c069f0fa108a27224c"),
    ("rgb-tiff-c2-ptrue-mtrue", "26e4ab46dd435c5312089fbb1585e650d089d9100ccadec9a864459d749f9947"),
    ("rgb-tiff-c3-pfalse-mfalse", "2e665112c0e57e6bb5d1e17e4c4423ab92352a035e7654666590e20c6c8d13ec"),
    ("rgb-tiff-c3-pfalse-mtrue", "079d48f39fac6505aabfb1cfd0c979a63a3f5b5587aed8f1b123433368a905bb"),
    ("rgb-tiff-c3-ptrue-mfalse", "46b6b1bd057e5ab51a2b359cbb854b9564062da33c263c646eeee0ebbd9619b9"),
    ("rgb-tiff-c3-ptrue-mtrue", "6d2b99d2f66d82aefc225e95b0e0c728fb765e3fbd63a993232a81cb73a6f9e3"),
    ("rgb-tiff-jpeg", "4bff511712d1771b0b06d1625ac13ca65a815bd302492b17bdeb7703d360787d"),
    ("rgb-tiff-l0-yfalse-tfalse", "6f9c3671b5f054906c48cef3c9a39ad49197e6fd1e0ce693f32dbd5973f49229"),
    ("rgb-tiff-l0-yfalse-ttrue", "6f9c3671b5f054906c48cef3c9a39ad49197e6fd1e0ce693f32dbd5973f49229"),
    ("rgb-tiff-l0-ytrue-tfalse", "06a376ee880a29f92a330ec640e53df06b915421f710898c2eeac187234cbc81"),
    ("rgb-tiff-l0-ytrue-ttrue", "06a376ee880a29f92a330ec640e53df06b915421f710898c2eeac187234cbc81"),
    ("rgb-tiff-l1-yfalse-tfalse", "d4f3bfd9d9e1c035f0e8c83df60bdd5aa61ce18b7f1d750bd1c907898b0a9768"),
    ("rgb-tiff-l1-yfalse-ttrue", "d4f3bfd9d9e1c035f0e8c83df60bdd5aa61ce18b7f1d750bd1c907898b0a9768"),
    ("rgb-tiff-l1-ytrue-tfalse", "08c512170baf22fb0759ebff587c67b0a3d248e16cfa5e80fdb2f36caab41a6b"),
    ("rgb-tiff-l1-ytrue-ttrue", "08c512170baf22fb0759ebff587c67b0a3d248e16cfa5e80fdb2f36caab41a6b"),
    ("rgb16_layered.psd", "313d0ab732fb8ed655b4dbd707a2bf01ddf7ba76858f5076431f98d61f28acef"),
    ("rgb16_tagged.psd", "313d0ab732fb8ed655b4dbd707a2bf01ddf7ba76858f5076431f98d61f28acef"),
    ("rgb32_layered.psd", "b7bd70faa538cd140d1fc79b6a394d557a6c5b0c1ba50bd1fc23b368c3a8f967"),
    ("rgba-bmp-1", "ca8b446ef491a7ac93a305ae2103c5cbd557efe0854c462992b4ddb08ae9fc45"),
    ("rgba-bmp-16-adv1", "d420e3eb7afa0328caf054f8564d96c5c5a1c7b5b19c0f352f4943d3646a00a2"),
    ("rgba-bmp-16-adv2", "b169d295f456dbad66b95908c183f655e2da11281dcf7eee3d76168d9318d821"),
    ("rgba-bmp-16-false", "73c23486a6a9ccdadb561fd63b9592475acf0d95453a2e42328a559f4930c8ee"),
    ("rgba-bmp-16-true", "5d3d9477ef995f0607aefe43b3c9e64d082ee0f7fde101e16eb6038222c44deb"),
    ("rgba-bmp-24-false", "9536c9b67bb188c763c8e2027eacc388a921999516fb027910be893becb306a0"),
    ("rgba-bmp-24-os2", "38e0dcafb7afa6efcebaa1fe6f00e284befabe7639569dc5a781484f17999058"),
    ("rgba-bmp-24-true", "c4a7017325964fb2b814574890044325a7eb987733b20fbcfc48fb182f43462a"),
    ("rgba-bmp-32-false", "33e949fd01b44f4b85e2bf2038808d9b1b47b3c232d9df549648055a6b40a575"),
    ("rgba-bmp-32-true", "6ca719aec493aed4eb09053b8458975a5e6def2248a0629342c6045030b88e64"),
    ("rgba-bmp-4", "d35d8c049c1269674fb1e8742adfad2dcc2fd7312f8388ca38663f723bf66e67"),
    ("rgba-bmp-8", "64a141f455ebede1ca3c4534e35fefa0e66bb68f3f35e84c36212a195c712edd"),
    ("rgba-bmp-8-rle", "776f161bb1d028896422bedef9e7cd9f7cedac019dbd4f307329f240fd84efde"),
    ("rgba-cineon", "d407bd3c54f32c8a57d40cf1a44c53b6825a985f83355692d0a061d9da07fe37"),
    ("rgba-dcs", "a46a2e7677ffb03dbe6f806f86c5fa06cb5cd74a33029e5bdb4bdbf8e56570d6"),
    ("rgba-eps", "f2d6ea0b1121d71ab3ffc78e3ec856c6ccf084fc7b0c5d0fa920a7298ce6301b"),
    ("rgba-gif", "0ef20fe54884109cd8253d50d8ba0fe5afd3c11ee8529541bc4486fe5b2d2755"),
    ("rgba-gif-i", "6e0d09916faa9a6a15c3dffd141a54a1273d0056e9d66ddc66f281ccc181a049"),
    ("rgba-png-0-false", "4265247ec279a2c82a29ba05a849ac7009cb5fe58a88d5349e59dd35130652f7"),
    ("rgba-png-0-true", "0f18bb361f1e5a57d5bd20edd0037c48e7f83876e4bc237f452ce459cb11184b"),
    ("rgba-png-6-false", "9e6ea1419727e8aa985cd52bd3beae2dbba9a4f9d3daeb3b32d854e1d85c2f82"),
    ("rgba-png-6-true", "41abc3ca1a32d79347ae549238b56a02c8b096706153df952267fa7c09881147"),
    ("rgba-png-9-false", "c6a34a164336d32d75be3de77ce5314b648037daf4993aa3095c47aa2261069d"),
    ("rgba-png-9-true", "790b2996e426cdba66e5ea5cf019a0fa10899333a6bdbc78b32ad49c213bc790"),
    ("rgba-raw-0-false", "1494d4d96cac6cf05d25bbf59a72a421b239aabe03df77f1e285acd61ee2ebaf"),
    ("rgba-raw-0-true", "076a6a8be5fabb838bf4fb02cf745bec7bd883fc382fb694fdeb53f74c86ae26"),
    ("rgba-raw-37-true", "64f68b820a41b79832203fb2368a579b2d5ce2a7ecbc79de52132804ba312543"),
    ("rgba-raw-64-false", "2dc6821638062bc1cea01d3a0d9fe5298ae3b1895135395272951d5624ed8d2f"),
    ("rgba-tga-16-false", "516672ee7b5a78cec09207710e48ecf5a2ef6514ea95fa6a362932e86ce07033"),
    ("rgba-tga-16-true", "fd1a03ef55f1fef4a5622be235401c6c64482f68ecb6fe0a7ec6b5718691a7fc"),
    ("rgba-tga-24-false", "5849911c0afc9c14016ca6aea084d89fdebefd79f0903b5a548afb680b73213d"),
    ("rgba-tga-24-true", "761ea0de7e9ef4ff20022ee896effd8b269f0a00a763de2309d975b5651cdae2"),
    ("rgba-tga-32-false", "6d7324f054eb8b477f2e14dcb688feee215e60f52b49c09d7f4e1325f31de16f"),
    ("rgba-tga-32-true", "5d619dcfff40426b5676aeaa6b6a1ffd7623dc63a94943defa685217732e7fbd"),
    ("rgba-tiff-c0-pfalse-mfalse", "60d6f9a55d98896b47b3ca8abf01a52cec058a39f3c433cc2201ae56cd079640"),
    ("rgba-tiff-c0-pfalse-mtrue", "6f47cc55e8eb30de6b6304cc6ce378a9078f6bc2f82167fd2c567175759383f3"),
    ("rgba-tiff-c0-ptrue-mfalse", "46908a568d47ceee5046d7c22e85dec4d324ab91769ea956a7ee08f23781abfa"),
    ("rgba-tiff-c0-ptrue-mtrue", "a783f9c54b23470a4d4f94e6d643fb12947b113f5117414c3c276b87af972a87"),
    ("rgba-tiff-c1-pfalse-mfalse", "64f521d6ce31e9a8a1b203d22c26d656d555b05f060d7d4c1ef35453ff82d43c"),
    ("rgba-tiff-c1-pfalse-mtrue", "14ea5a14bffb2ecff628ba24012f2f1f5720e1e7c7567411d9ba7325dae30dac"),
    ("rgba-tiff-c1-ptrue-mfalse", "075a2c57e721602c5a156e512c3f1a47049959e80cdb0fd8f668c6b29a5c8225"),
    ("rgba-tiff-c1-ptrue-mtrue", "313e4da035eaabd88d776107bfd31d7d8b7a474f10f39877a6690571f33ee9f0"),
    ("rgba-tiff-c2-pfalse-mfalse", "ae627da6ff1c122cdc70485880e52e0c775f096b9291fac0f16bad9cf1a9026c"),
    ("rgba-tiff-c2-pfalse-mtrue", "82a71367ab6d26bd7633d7d1355c8a4a6a1f9c444905873a8f322bdeda2234d5"),
    ("rgba-tiff-c2-ptrue-mfalse", "ae04a56f7416353e711c04dfbd36238e254b5f5fdba07dd12ab2c01e0b525e93"),
    ("rgba-tiff-c2-ptrue-mtrue", "828ee0c0ec0c96e87c4ffa7a98ba2e8cfddbf1695371e92aa58f8b66544da152"),
    ("rgba-tiff-c3-pfalse-mfalse", "11e757c2aadd88e0e56b05d883ea2e0aae6370926ecebb8904bb12a1959527e3"),
    ("rgba-tiff-c3-pfalse-mtrue", "e8c15373d9e1b45acb4a697addfa8f1e024a918a0e9bc65d601cd55a985af2f4"),
    ("rgba-tiff-c3-ptrue-mfalse", "8bbb5be6f8581cad496a2ea65ea3f5943c231970dd3c5672c9483d2a7e403dd9"),
    ("rgba-tiff-c3-ptrue-mtrue", "3dfdfe98800d9657e5bed64bc3798a3671e0da968232c58f93f76f60925aa5a7"),
    ("rgba-tiff-jpeg", "b969edc25deb1c93984d4423da651eeefb258e40cd49a207e575fe7331f85f1f"),
    ("rgba-tiff-l0-yfalse-tfalse", "03f708117eab14df32469b4db135e407d07a3fc90448619cb92aa1d4d8f77d1a"),
    ("rgba-tiff-l0-yfalse-ttrue", "f390361657fbd14bc66b54c3ab38bcdc443d22ff2d688382a957b050bd8a4dd3"),
    ("rgba-tiff-l0-ytrue-tfalse", "f2d00ffe0040fad762aea91f349d9cc32f35d7bccc516b2dbc71c0574bd558b7"),
    ("rgba-tiff-l0-ytrue-ttrue", "858c4e44e0e28eaa4d1bd6016e3374612c92f3550dd53b659e2a9f8c440106d8"),
    ("rgba-tiff-l1-yfalse-tfalse", "09a11ca137eb7c5cd501cd8e0497dfc458aaab16db2663315b3792d35e79c6f4"),
    ("rgba-tiff-l1-yfalse-ttrue", "f938adce76b6e252e655fceb270282e885d05101fb1c5ee4427a6d84955ac2fe"),
    ("rgba-tiff-l1-ytrue-tfalse", "ba5a4df02cdf47d9cc103d3b8950f05065db75bef8bb56e436280bf4e8c9825d"),
    ("rgba-tiff-l1-ytrue-ttrue", "bd6cee18b31643d58d4ff25bbbacabefd9aee03d9ed8794520b5d92af1773523"),
    ("tiled-raw-raw.psd", "c7bde0865cae0f98627ba9deb76b793af06930608125fa77336538f87fdd8553"),
    ("tiled-raw-rle.psd", "f5610a5206824418aa8ff1805f55c3114a370d68e1718eefb47f2c9a43d4ce9f"),
    ("tiled-raw-zip.psd", "eeb77fc1298d8472b56d7f9463aa33c02150021d139c930c1a6e9866e6df19d5"),
    ("tiled-raw-zipp.psd", "720c8ae6059c56e2eec3e2d9b90021dd8eccdf47bf78a8937c862ae1fa5716d6"),
    ("tiled-raw.psb", "82b3b9d1532fa970d38efc5c7e887175fd29637eaab997160f62d9d6dacc1ca5"),
    ("tiled-rle-raw.psd", "c81adfee05d796f11ee23149024d1e362f7687f833ff5a44162504def0d76f64"),
    ("tiled-rle-rle.psd", "712fde6c582446b16872f219ca7e2ea813a5963f78d35104afa595db9bf63d4d"),
    ("tiled-rle-zip.psd", "040393dd01590684a7eb40f5ec65eb88b40263fa3af3c29f052af347ee0b16a4"),
    ("tiled-rle-zipp.psd", "b9073427e25428cfe8eb0310b9fdaaabb2d03c40bf7a9a00e5b143354bd7a43d"),
    ("tiled-rle.psb", "b2d38156b8307005155ac8e2d29e6fd585f4021feef1ce6240c311e23faa72c6"),
    ("tiled-zip-raw.psd", "3032ae97deb3cf87aca78a8edaf44f39c45234ef4699939246c83ac324e36ad8"),
    ("tiled-zip-rle.psd", "7a3f744926143143d5d6291e2167bc7d511e0b0d052046e22573664fd6b13c0d"),
    ("tiled-zip-zip.psd", "8697f75e88f146558d55aa47ae15056ae8c4ad5e33af6f38025f3bdc8915b44b"),
    ("tiled-zip-zipp.psd", "43cfba07a9f4bce1406a3bbed6432729de3054d9eab598ee92d3ad56556d2ef6"),
    ("tiled-zip.psb", "183ae3b1a0a5dc6bd8b78e5ea79b63d24d747187e74427afc702cc978b6c6da3"),
    ("tiled-zipp-raw.psd", "48f36b8bb74b60f366fcf9d16cc74f687c0d84acde3d8cd1891a14c7aa032bb7"),
    ("tiled-zipp-rle.psd", "c2de77af5381d73b2205d4507ec8880940077a224b419b5e3bd0e5fac7608006"),
    ("tiled-zipp-zip.psd", "944442ccc9b5e37ab9b4db85158e9880ec6dacc61a20b17004fd99852ed9f37f"),
    ("tiled-zipp-zipp.psd", "4ebaab9a84448e369361210e242d5b949eb0723f8f16a199f50d6dcd8c19c0c1"),
    ("tiled-zipp.psb", "00220f3bb622c14e43b0f230da0d0cb46af63d4ca7b18b2870df0421f1862a0c"),
];
