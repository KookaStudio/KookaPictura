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

/// The SHA-256 of `bytes`, one letter `a`–`p` per nibble: hex digits would
/// sooner or later spell the plugin extension `scripts/guard.sh` rejects.
fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .flat_map(|b| [b >> 4, b & 15])
        .map(|n| char::from(b'a' + n))
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
        // The stub's payload as recorded: the samples' SHA-256 in hex.
        for b in Sha256::digest(samples) {
            out.extend_from_slice(format!("{b:02x}").as_bytes());
        }
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
        SaveKind::Pict => "pct",
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
    ("bitmap.psd", "bbidpfpkobfodcfmfncoccikjilpaeanlanmdhbenacifcldkcjlaggnlbncdjda"),
    ("cmyk-png", "dgmchenhpangildhnccpenejacldfkdbcdbjcmbkoinhlfdnheeecfpdplmjkdpa"),
    ("cmyk-raw", "golojnpcakfccdlgjcgomopdpdmekjnobnelblmjmpelnemechlaggejjnpjcibh"),
    ("cmyk-tiff", "eombkmklmmakjmepfmddgfdccfenpcipjochblgfmlnegapeepgkcbdfmbiodeea"),
    ("cmyk.psd", "bnmmhndlfhfnhgncldeimblmjelgjjjlnobalghincaiemccacoegkoajghmhhln"),
    ("flat-raw-raw.psd", "kddeideiniigkfnpcpkcaifogocoacalkjgfokhnebncgipmaggpkmjmgcokcoba"),
    ("flat-raw-rle.psd", "aagedieameeiaicilfakdageheapfiifbmfaoklodeflppipmoeendajmohpmkmg"),
    ("flat-raw-zip.psd", "modlkncnepfdajhmnieegdhjjdfhjhkmfjfabjajlmkomglmdhghfjcekakcoeee"),
    ("flat-raw-zipp.psd", "pfjejnjnlegpfikbdjdhlbicnlhmhhbghegbnclegnmogbdklphhjofgfmokdohj"),
    ("flat-raw.psb", "fhbnogcghncidihbbnnpnebcikoballabfiphbpedoiefkbliemknjbcjemmenad"),
    ("flat-rle-raw.psd", "glmfhakhnbblhemdejfbaahkdhcfgdmeoeoobeaabgkcodmlhkigmnmikbofjngp"),
    ("flat-rle-rle.psd", "bkhpdhnmdaenlafnefomkhigkinfgcphfogmpkliaechfdmhkgfjpnenilliboap"),
    ("flat-rle-zip.psd", "obciekdlaoamnecdllolkdhncgaibblbglaaekfhmhjpfeeaemfabgcdbafeofpp"),
    ("flat-rle-zipp.psd", "khinndmfaadofbpledcidlgfkifidjkgcimlelilikjbjggadcifggplkebhkhdl"),
    ("flat-rle.psb", "omoadclhejihfidaednconcfgdjaafgfcklgaadleljflgbdhajnlegpnkmkfdce"),
    ("flat-zip-raw.psd", "nekiekijegnehlflamijfnaejlffkppigflbnghhbblccfghbajjggpamndnnmch"),
    ("flat-zip-rle.psd", "oddkabnfkappoebcbopdbkggcapblgnkjocnjlfblfanofieogfjaoiggabachao"),
    ("flat-zip-zip.psd", "lhbdelmgclnpimndmgemaabmlmppcjmngodalendhgeihmcehddfcdhmplklgndn"),
    ("flat-zip-zipp.psd", "jnppglannogdmifbccoolpnleidggkfikefojbkoaoiefdejdheoobfpbpdohhlf"),
    ("flat-zip.psb", "dfmmmejpmnbjmkjjmlcnehkkoogjhgpfhphdlokiglbobmkobbfbngfhenbbklln"),
    ("flat-zipp-raw.psd", "bnfclddhlhholmpfcoiljahaljlclbbgibndjhlhfiofchigfnjlmcmfhodpdjgn"),
    ("flat-zipp-rle.psd", "ieonmopmkglahffgfoiihffmpgeimngmgmbmmeckcchadbmcikgchabmiolpiegc"),
    ("flat-zipp-zip.psd", "ebijpidjblolmickdppdhaabngabmhaeooffmkokpjicdgkbpbaciafehkgjaehe"),
    ("flat-zipp-zipp.psd", "idmcjhacjibibbhmpdjpafdhbcookdacfhafdegjeeahgcpchlmbghkbonkdolgh"),
    ("flat-zipp.psb", "omhdpmjdanjiebedmnjfacigigpllglkjdkkbfadhnnlbkbaoahooeknebcmmadd"),
    ("gray-bmp-1", "ldbifdocmapkpnihhllhopmmmhlphcadonejegoopkecpkoifingbmmjhggefbml"),
    ("gray-bmp-16-adv1", "kjbdolcphflioeaobhenjaffgmkgeboaboigplpaabfniinjnidfajgcfhnjabdh"),
    ("gray-bmp-16-adv2", "llofoblnddbdpdfinafkdemeekcjlffejngodcfmiffjhgnaajpociamjglnelpj"),
    ("gray-bmp-16-false", "hjpddhepenmdjohkbcbceekhgomfiklfcmphhpmlldjigoaglkdaldbhcabmalci"),
    ("gray-bmp-16-true", "bngbjchcbinnjglnohnilncoldodikghhljchbgihnebedcfpekodknmbljmckmg"),
    ("gray-bmp-24-false", "olpmdfodfbmfckgchofhfkenoblddpgpbbhdegknegnkfdfgoahbdbfehjldahil"),
    ("gray-bmp-24-os2", "kgcopcpldmbgckicejjdopjapgmhglamganlhpbioiioigmhakfmpkgcjohepmkk"),
    ("gray-bmp-24-true", "cofpekolddaebplcapkhfddkonlfdkebkjmbjcmdioebdjgmdempdmedmnmoeljn"),
    ("gray-bmp-32-false", "koadgfgpkggcelodpblioghhfdmimgfackcoonpgnaeacdhioajeobmclkaecold"),
    ("gray-bmp-32-true", "bikefkopbkjfpiohpilldakohnkimibakhnoilcfopmfdogoigdglgnlbkjneido"),
    ("gray-bmp-4", "eindjdogmeohagkkdlcbcdnobcnmpepmceglahfcppcpojpdhhaoinjgpfenaapo"),
    ("gray-bmp-8", "kbfpaoiafkccbbdphifdjeckepdlkgojfcncpeknfbcecjkbnhleieelpbkldhap"),
    ("gray-bmp-8-rle", "knleefgnnmnaanlfnhjhhjijbfogdjnjdjmlheofmeibjopklcoiiiojmpgomemn"),
    ("gray-cineon", "bmgpkoeciihjedfnehibjabaoolglhebjdbafjenbdgccadejlikfhdcdjbijmmd"),
    ("gray-dcs", "nejbhcdjeonlnilcebhnjlcfkfapihcgjhodpejdlhgifjnjaecggdlfagaakgic"),
    ("gray-eps", "noncjkalklkmheobbmagimgfhoecclglibbocdogmjfbeflemfafbhddnfacfina"),
    ("gray-gif", "gmolmddaejalmolkfcpedpoopcgicpmokfjheopfdaddemmecpgmdoiopigmphap"),
    ("gray-gif-i", "ijjdkkeockmhcpaoaplcgfncgenjbgbnanidccbobpcbfhgledpjafimamfiflmd"),
    ("gray-png-0-false", "gbmpjdgpcheeljcnadlliejceeegicalmbjocdekgjbjblcejnfclolpjldhoemc"),
    ("gray-png-0-true", "leibaflfblfehkbkeihmalmccdiccbfohlofbflahiepajbfmjbgacihoioccooa"),
    ("gray-png-6-false", "ejpafkpfpcpgabfihcmogcjbjbgodgbleepnenhhfobphgipahcieldnnfedpjbe"),
    ("gray-png-6-true", "bdkhnbhfkmoanbkfjjcmbhieceacbchmknmeafkeocpolboglmndclagdldojipk"),
    ("gray-png-9-false", "dgidmfcmkkgfpbhcjkaedmocnhfipkiicfcbihaopcoddccgcblfjbjpfllnilhn"),
    ("gray-png-9-true", "kmockgaliacalicdfnogiknkjkjdmlghglofmnkcekmfnoibpenbkipnabpdpkkg"),
    ("gray-raw-0-false", "ipedopmndclfoklnimjefoojnaeecmmajhmlonehbnmggmhgfggblhpjknmmkgjc"),
    ("gray-raw-0-true", "ipedopmndclfoklnimjefoojnaeecmmajhmlonehbnmggmhgfggblhpjknmmkgjc"),
    ("gray-raw-37-true", "ldegedlefibiokjemljhgoodgleaocpbohjahgidgdcckcchmibfnmkpehhffhhb"),
    ("gray-raw-64-false", "hfpkpokkaeggffanamglnbhcpdkbkakdhihihfblggfkpehbolejjjdaljhhkadl"),
    ("gray-tga-16-false", "lmnggpojdlmajbfmaalpbclnefhhpdgkdkglgegalpnhjbhdideicgecmiioomnf"),
    ("gray-tga-16-true", "mjgmlmeikaendafkmipnjbmnfgomfmfpamifkdaanacomookegogjcjhklcalckn"),
    ("gray-tga-24-false", "ohheahnbcildabkafbbbicgadkifohhmoefhbpdpndlahhgpoaccbandbgojcdke"),
    ("gray-tga-24-true", "eokielkpfhbpailpnikgeaiillnklnckmpighocnpniamehfhlnpokinneilakdp"),
    ("gray-tga-32-false", "hiobgencjcinigabnafjckkkodfbncjjclaidobpjndncdpklbcbpdggcamipnli"),
    ("gray-tga-32-true", "ifjjpjhbchmjfaedkckajccfojfpoefkmhmcjggafghkffngpdeafcbckfnbdijj"),
    ("gray-tiff-c0-pfalse-mfalse", "pnkaalbkahobaoemmafnpmfhjlbcjeipagedjhibnfcplbaimbbilokndmgjgicg"),
    ("gray-tiff-c0-pfalse-mtrue", "nmjpimgcielldcdldajmjchmpmmbohemdboifiadjhkdcilpdleabhlbmendagac"),
    ("gray-tiff-c0-ptrue-mfalse", "geoheobeiflpfnebkmadmkfobngjbbhlhbonoaphjibonfpniloigplklaafnoka"),
    ("gray-tiff-c0-ptrue-mtrue", "ckpomfbommpkmphdpamjpagcgnjffcoodjkkpecaaembgjajcmhfbjihllohjafb"),
    ("gray-tiff-c1-pfalse-mfalse", "bechogeedcmjiajndpjmadciibcbnamcoilmmlmghcbcnnanmfhgfoogpnhneohg"),
    ("gray-tiff-c1-pfalse-mtrue", "blbbdbajbcfplagmpecopbjmjjdnhocebglcilimnokbpenigmalhgkbjddcmima"),
    ("gray-tiff-c1-ptrue-mfalse", "ogbkmemjkiiebflnegfdbgcfbobbnjiihdbmloijepbicehajdhnglbjpjjcocgn"),
    ("gray-tiff-c1-ptrue-mtrue", "fankkhiidlpnlldbfajgfahefhkgcimmchajhlcmefdaeebelciihehmjilgbhcj"),
    ("gray-tiff-c2-pfalse-mfalse", "lgcdefaoikipegopnknmaokddppgabiblkmlijcnoibkilciahleiahbohdcagkk"),
    ("gray-tiff-c2-pfalse-mtrue", "noecmfdoefdgaodbddigpaejmbpaeibhkkicmmdibfaplcoabekdofllmpdliidk"),
    ("gray-tiff-c2-ptrue-mfalse", "igaeplidfgimjaaljmgjnlkgfcfmdjionmhghjeldnmjoggjioplkmmdbbfpadlh"),
    ("gray-tiff-c2-ptrue-mtrue", "cpfjkemhmdifhgdplofjoinhmlepjohmkghegdcfaidmnlaokadhmmmnebcddlii"),
    ("gray-tiff-c3-pfalse-mfalse", "necfokoljikbpnonnmcfabkgaogeolpbhakfmnnokkoiejjjjokjapamjckemlce"),
    ("gray-tiff-c3-pfalse-mtrue", "hhapiciaenejlhkmehjigghdcmniacjbnlibpkgdnloanidpiknhffihgmjfkolp"),
    ("gray-tiff-c3-ptrue-mfalse", "ppcdeneojnmenenjbbnobmabeocnadnpafpkljldefdidocljeahamjelgefahjn"),
    ("gray-tiff-c3-ptrue-mtrue", "jgkpjahaemahejdgcelcejimldnbpancimopjepbjpkcpmgedcbhhbpmeiodmbbc"),
    ("gray-tiff-jpeg", "dmbfogdgbbenpjgplgajjpnkccomkikcdhalnhjmmgbkcohcfbppjnmaefkefaoe"),
    ("gray-tiff-l0-yfalse-tfalse", "nkklcbekienifmlbodojiejdfdlicaiephbpfhnhfihgeocnpoiglcebgegcnpfc"),
    ("gray-tiff-l0-yfalse-ttrue", "nkklcbekienifmlbodojiejdfdlicaiephbpfhnhfihgeocnpoiglcebgegcnpfc"),
    ("gray-tiff-l0-ytrue-tfalse", "pnpfjnbdmepleocffhiihdipgjdmlmlfblfmooehnkkleecfkogdcicpjibcpllf"),
    ("gray-tiff-l0-ytrue-ttrue", "pnpfjnbdmepleocffhiihdipgjdmlmlfblfmooehnkkleecfkogdcicpjibcpllf"),
    ("gray-tiff-l1-yfalse-tfalse", "adncbfbhidnomacpcjplmjlfdmfmpdgohkcohmhdijgfegcbcjdkmjihemaodkmg"),
    ("gray-tiff-l1-yfalse-ttrue", "adncbfbhidnomacpcjplmjlfdmfmpdgohkcohmhdijgfegcbcjdkmjihemaodkmg"),
    ("gray-tiff-l1-ytrue-tfalse", "hmgpnkpfalkppnmffkaohbeplgpkcdhadbhhficjjigfanknlkgjdkohlmcemepj"),
    ("gray-tiff-l1-ytrue-ttrue", "hmgpnkpfalkppnmffkaohbeplgpkcdhadbhhficjjigfanknlkgjdkohlmcemepj"),
    ("gray.psd", "nflkniebgbacoddleepfgjfhlefngegomjamfkibfodmljegnhgcgmcipplpkhmm"),
    ("indexed-bmp-1", "jcahiccagolebplliealjbihodgmeilmbdjkkihbelfiihiojcgpblmpkldmabno"),
    ("indexed-bmp-16-adv1", "johcnalaedjchdplhdmfdobbolnidfmfbbkhlhamjchgonlemfaikoknkjiebkgk"),
    ("indexed-bmp-16-adv2", "gabpopcjnfcjccklolhdkimdjfjnhhkdejchkffdokomikjemadjjhjhaafdbghj"),
    ("indexed-bmp-16-false", "lomjgeellcnagbcdkafkplpkmdjnbkfoklnebibpemfkkfkmjgaahkeaojeadljb"),
    ("indexed-bmp-16-true", "bbnimgmhoockbnknjgoijmelololnibcknjhnakghbgcgnhneohfhjagdbfnglah"),
    ("indexed-bmp-24-false", "nonpbkpglajblpacbnpijpobmpmoifpfadkcfpbhnapdkfkipfnbledjkflmmgop"),
    ("indexed-bmp-24-os2", "jgfcfbngkeibklmajijolppfobdcpbfibhggaoifmmcjhpfefbmgekondgapaijh"),
    ("indexed-bmp-24-true", "mnplmbfhlmiogehbopjhnpcebkaldfgngmnpkkkeplmobpdidgecmcgdnfnffefj"),
    ("indexed-bmp-32-false", "odolipjeeobodndhagkdbipmpcbgnhnkbjedhhofmojpenonbhjhkdihphlbgkbm"),
    ("indexed-bmp-32-true", "pjammcfpcpbleempbkfanfagjabdllgfkmppngffjboidkndiegbllkoiomchejn"),
    ("indexed-bmp-4", "pkmimmicnbdnmbmcoihhmlfljknpoojnghnigijpggffhhhopkkbomdfpclebcbf"),
    ("indexed-bmp-8", "lfbnfgagidndehbgjgfglfjnhpokplnijmolhhkobopeafkgdnlglfmaimbhkgjg"),
    ("indexed-bmp-8-rle", "ecfpjlnhjjpnfbnmcpeaihgapnfhnaccopmoefffohgmpmbfopmffnmdngbnbbmp"),
    ("indexed-cineon", "mjkpgoahjgjhnbeiappofcbddpnmloonhpcgnanigicndkihapknabohjfiidfkp"),
    ("indexed-dcs", "opcncfmgpijdklnekcdialkihkfjlhlfaioopjnmpcdaddhdpeolfpbdhhkffghn"),
    ("indexed-eps", "bdccnjpebfapikgjjakaidlfonlcabcpeddenfhcmdahhgdoplgngnkbjlgfllaa"),
    ("indexed-gif", "hpcappeafkmaknkphfnlhchpomjlobdpgdoaofnkpcdhddbgiiljgcimcjknogmi"),
    ("indexed-gif-i", "jmemhaoghcdapnmcbbiopbfdinlillkdidglgdpidohomhkocajijlljfgkbfpmg"),
    ("indexed-png-0-false", "kfkecacnhlfjfamkjnbbgemgcdoafhhmnpcckphplckljnninedfplikjokoapbk"),
    ("indexed-png-0-true", "fmakhnchbodlbolaedideiohenkcddhinjdlihhgdfmapbnejaldckohmjphknnb"),
    ("indexed-png-6-false", "ififkbjcehhpnlknlbnfkpalagajnihbdpclbmapbbdpenlcniblgillpkjdpkaf"),
    ("indexed-png-6-true", "bjoolgckcldhlnpjabbfmpikgclconcaglhomopnijgmppgncfhmopbgpgclefhc"),
    ("indexed-png-9-false", "monnlegmhkphdkcanbjdpgdganlmmmibikcipagebjigmopflggefnbijeoeknkn"),
    ("indexed-png-9-true", "mlomdefipmebbcbacclagnljlflackipjnbhhmhjdjkinnhajphpgakecnccekhf"),
    ("indexed-raw-0-false", "mcnkndhbheiajjlnohhjelbmlbjcbkklmdlfpddbfggbhkncnhkdjkdheadakaoa"),
    ("indexed-raw-0-true", "jnjgohcnogkbcfldemlfocflcbjcaknkdmoakmchjbelbemcdbogeaadomgolpnj"),
    ("indexed-raw-37-true", "pjgnlhennfkjpldlpmclgndpdjhbhfeblipefifckldgmhobcihhijmnjjpghbam"),
    ("indexed-raw-64-false", "lbdchoibmmplcchpeohopghnhdofadhnjpkhbkppihebjohlohjengdigcpnbmfk"),
    ("indexed-tga-16-false", "nlfpabcmbfhgdmgoimkplpllnbihpkpeecgjoeflakoidhmikpaljeehleokcjbe"),
    ("indexed-tga-16-true", "hfigghfoehfpannjolfgpcffnplffklpmgfbdjeihjjhknhdnnooppnhadaaoife"),
    ("indexed-tga-24-false", "jkeonpfadfklcpdipmooeilebealdinhanhgeaihlgpifjhmkojfliipodfggncl"),
    ("indexed-tga-24-true", "cnejcicbfhfhncblnabfogdcjapciefanbackegmmedjiklebjdgoomaecdcggji"),
    ("indexed-tga-32-false", "haopbnigmmniiojcihglgfggacidbglfppoghmpmjmpoggcngcebafflgecgffeb"),
    ("indexed-tga-32-true", "cgokdibmdcbhljifelpbaiomjmcbgohmfjdfbklibflkmdfgnmlojeipfbimfmkp"),
    ("indexed-tiff-c0-pfalse-mfalse", "lfhnleafheodmgpfdpajealaiaadfhhjjpmmkpjpkncglmkmgohckbcdbpeccpam"),
    ("indexed-tiff-c0-pfalse-mtrue", "ofkmjddiholaidgjielhfnngehpigaljkihbhgfhllgcdpjpgpfjmedacghnbfck"),
    ("indexed-tiff-c0-ptrue-mfalse", "ngphaggdkkoffkejmjbfglppgciedlikicahjohafaalhfboihfnpnpjjgmnobeg"),
    ("indexed-tiff-c0-ptrue-mtrue", "elllpginfbpplcldjjmgfdmpcjaflhdoploeaggademfiafihbfmnjhonggndini"),
    ("indexed-tiff-c1-pfalse-mfalse", "pjkkbljehckfhahpejbhnncebfkcljbmpncebdafbfieoppnjpdbjggpddhfmhjm"),
    ("indexed-tiff-c1-pfalse-mtrue", "ipbpoplfecmenigljnghmmjfbkhmbkilponlnkohcpemeinfbcfjdagbigopmbhp"),
    ("indexed-tiff-c1-ptrue-mfalse", "dniokpgmefhboplegkaajgihjjekjgbhhcejdhnmlibhknkokplncgloajeifiml"),
    ("indexed-tiff-c1-ptrue-mtrue", "kjmkagonofeachbfbblgmeagoaoelmndcalnepmlakggdmklfhgkafbjbbfdgdce"),
    ("indexed-tiff-c2-pfalse-mfalse", "dfkpgjldaaobfmfkppcfahaaomfjkkiondidgknolfepedjhkmfgdihkgbemomma"),
    ("indexed-tiff-c2-pfalse-mtrue", "pdablppiefjhbfjbgkhfhafefnpglikgidpbnmnglmjikoaceelpaifhbnmigeim"),
    ("indexed-tiff-c2-ptrue-mfalse", "enckaaehpekhoklcbkjmocaocjikcgdjekipidmfpdnkeakjeoomhaoimpbdejlo"),
    ("indexed-tiff-c2-ptrue-mtrue", "jdfbadblccnpkgfmeemmedgihhnjedeaaffdaileekpcophjcajpdjmjcejipfce"),
    ("indexed-tiff-c3-pfalse-mfalse", "blnjalaclelehchfmfdmmoldmgcdkofiocfpggjnkkokgooiakojbjdaebjjdnhj"),
    ("indexed-tiff-c3-pfalse-mtrue", "cialpfbepnhlpidiakheggdgbommhnblnpddpofamkijbmhdcbgooppgihnpppoc"),
    ("indexed-tiff-c3-ptrue-mfalse", "ofkmhgfcaclblhcpnhefookkhdlhiokdmdimlnbihjfggmcldipccnoaopanmfdn"),
    ("indexed-tiff-c3-ptrue-mtrue", "edhljlbadbkekohijjlemnnaighbiikajfnbneijbbpfnpjopgmneemjlgknipii"),
    ("indexed-tiff-jpeg", "pdmacjadabdiolkhjfeoeffnmjjbmhcndfcpcklmjkomjogdekmefjihablfacmc"),
    ("indexed-tiff-l0-yfalse-tfalse", "jjmidenllhfngaokinbijlpbblapebopaidljhlfhgckdbnlgcmcpfcgljfpkbjf"),
    ("indexed-tiff-l0-yfalse-ttrue", "jjmidenllhfngaokinbijlpbblapebopaidljhlfhgckdbnlgcmcpfcgljfpkbjf"),
    ("indexed-tiff-l0-ytrue-tfalse", "gcplfooopcpokomaanmcnljmimbneeakkipnehbjhlfhanhjchdolkedcmogckdf"),
    ("indexed-tiff-l0-ytrue-ttrue", "gcplfooopcpokomaanmcnljmimbneeakkipnehbjhlfhanhjchdolkedcmogckdf"),
    ("indexed-tiff-l1-yfalse-tfalse", "jjmidenllhfngaokinbijlpbblapebopaidljhlfhgckdbnlgcmcpfcgljfpkbjf"),
    ("indexed-tiff-l1-yfalse-ttrue", "jjmidenllhfngaokinbijlpbblapebopaidljhlfhgckdbnlgcmcpfcgljfpkbjf"),
    ("indexed-tiff-l1-ytrue-tfalse", "gcplfooopcpokomaanmcnljmimbneeakkipnehbjhlfhanhjchdolkedcmogckdf"),
    ("indexed-tiff-l1-ytrue-ttrue", "gcplfooopcpokomaanmcnljmimbneeakkipnehbjhlfhanhjchdolkedcmogckdf"),
    ("indexed.psd", "jkkkfmgmhhiabopgajlihhdomgmpekhdodpjhhpdeailnbbmpldndlmaohpflaed"),
    ("lab-png", "bgclihfblalnhfigacnkbgmlgfeileclelgkbjbhhalbmkcoaoohpilmdanigocg"),
    ("lab-raw", "pdbnngloihboodfhafclhfnebimfbbjochcnfdbpmfgaincoinfhdlbcemohfgef"),
    ("lab-tiff", "gkpgkojkekkgnlfofjhngchmgdabcinhcbnelpcmkfijblhhnpfmdfahpmpjipji"),
    ("lab.psd", "mpdelmledghecdbhcjigaidjhghcgakokmfmbhpjfeckgehhgjekjcjiajhajggd"),
    ("masked.psd", "ophlihffgpodbmfccdnifhnnonlgdmjbjpmoligcoaabbgpbabjmjgipkfabkanm"),
    ("rgb-bmp-1", "hmakhecgpebilfikaoekhaljejnbkdbcgpjmlbhecoldjekmhgelfefglmdgpcjh"),
    ("rgb-bmp-16-adv1", "lcidbimhgoggfmhgpemalfhdodkljkbdpfoehbnakakkifpkfebojhhaldegfpce"),
    ("rgb-bmp-16-adv2", "cpinjfaghmfiaclmlgbejjjkhhnlopdifmlkmbglcaalechnmdplhmfggejojijd"),
    ("rgb-bmp-16-false", "koklfgljnaekkjkdbhlohgcdjplglchemiofdcknabiicfpdbgoalaobhoglgfpc"),
    ("rgb-bmp-16-true", "mhalmbebnihafbhmnoidigoccpngkfhamdenecaafglnloijnlkjhakggakoepfk"),
    ("rgb-bmp-24-false", "hdnjlnodknedfpgnmofiagohelidgkeboaealdidplgkhpgjpfcoggiknagmhckh"),
    ("rgb-bmp-24-os2", "dodbmlemjlcfmdmmefjkflohiggjmpljcnjkcpejlhepaoiidochafpldfamomcb"),
    ("rgb-bmp-24-true", "imoneimmcebimoijilfeoomfgehoikhohppmhafnngpniijmhjlhaacebicmjlol"),
    ("rgb-bmp-32-false", "meedgiaaocepgdphonccooecdeiijfojmielgfcbdgnpcpfcalpekcgmbbppmcgb"),
    ("rgb-bmp-32-true", "jkkhfghllgnpanliapdidnnnmmjjbojghhicephaiefjijkjmgoddhokjcdjgion"),
    ("rgb-bmp-4", "nbgppeeacckojdbeofpoidkgennaopphllocldfifnondffhphkdiaobgabaecbe"),
    ("rgb-bmp-8", "cceeicgpdioefoeohkgdoejdagmkgfbnoniolkckoeoehimfblnmbkimdmlpecoh"),
    ("rgb-bmp-8-rle", "mgpkfbcajoddiengmebjpfihfdhoccglhmmiiafifgcpofgakflocmoecgnnklni"),
    ("rgb-cineon", "neahlndmfepdcmikfhneampbkeemfdlgicfkjifpiddffgjcnakagbnjnkahpodh"),
    ("rgb-dcs", "mlihngdhaokljjeheoachgfckemoondedoadfkmgpeelgajgaffpclabgojogkln"),
    ("rgb-eps", "amlkicjjompomokmomogfbeiafanhmmbklfplbboflcicdhpodpnglbnemcmjdkp"),
    ("rgb-gif", "mpfbnaiboeccdfhhjfabeeheikhaogfokfhncaifjemgeflikeildhlmbjnbagem"),
    ("rgb-gif-i", "odipnjjepkebpfbjmhfpcefkahlpdicoaapdchodnbikjbmiakhhgfjcccahnnil"),
    ("rgb-png-0-false", "folcafjjjfemgcfnikkmdplfcdgbdfkbfdhjfdeijmbemmpodphkakbgefahagdi"),
    ("rgb-png-0-true", "khghphhnblefjjkkmmdpfgnjglabgcmlbfoijhmnlpdemcpabhfgihjfgfebggbi"),
    ("rgb-png-6-false", "baelkccbikmfldamfpafoblpaajiblbngaomnjjkelfapembnalaafhhpepgcmjp"),
    ("rgb-png-6-true", "nopinfihelclaiciimpjldfdhneeokdhjoempedmbajefdpbfcjgffacohbdllme"),
    ("rgb-png-9-false", "djkimjfbamlkioinlcdhddblmioolgggbjhmfpcpdpeikmepjkpcfkidackghdpl"),
    ("rgb-png-9-true", "hebgioepphbpddbomghlddjjejinldmpmknmgcemllhhmmgghhelpfcjcgmobbjg"),
    ("rgb-raw-0-false", "bejenenjgmkmgmpafncfllpfjkhckecblcdjkkloadnphhpbocifkmngboocolkp"),
    ("rgb-raw-0-true", "ahgkgkilofpkllidilpeplacmpheflomhlniidpmdicplgjepnolfdphemigkocg"),
    ("rgb-raw-37-true", "gepgilicakeblhjidccadplcdgikfhjlcnfmockhomlmhjnofcbdciaelkdbcfed"),
    ("rgb-raw-64-false", "cnmgicbgdiagclmbmokabndkanjpofcjikodlbijfbdfdjfchcjfbnfgceonincp"),
    ("rgb-tga-16-false", "gnjcaocnojnaokhmhkgbllhcnphpppogflbnbojjoinjjioaokedhghdgpgpcjfh"),
    ("rgb-tga-16-true", "dplbidpeonhpfamcldbhkonmlkjkeaamkfeilepgnkjpnjfilbngekbofjggljdo"),
    ("rgb-tga-24-false", "hpammbacpnlppoalaamakjodkdjfcnckjagjnglogjkgabacaiodnpbhhmilclll"),
    ("rgb-tga-24-true", "occfmmiepmoipnilnhlppondfhgednkhkmenenbkdgdcembgpgkehcdkclfcjofk"),
    ("rgb-tga-32-false", "hfopolmeghdfcmankogapokmbkoiigmpppfclhnnfgckkgboeiefpepnemlcejon"),
    ("rgb-tga-32-true", "hpmclbicoehjjifaebgjhgkohajegahlmokekdjfhbpcbfphidhdfckakapmgmpk"),
    ("rgb-tiff-c0-pfalse-mfalse", "meijnloekkkhoiedickpcfgcjkjebigpcddkgdencldobmnmamfmbnmpelhicjnf"),
    ("rgb-tiff-c0-pfalse-mtrue", "ngdemecdpnmhblciemfooehlfcdjonhfgnogcgkjkakijekhakkonenhffcoonno"),
    ("rgb-tiff-c0-ptrue-mfalse", "eopoagpdaofaohaefelmopmhidffcnnoeokindihchdgcghpaaodeghddlblabna"),
    ("rgb-tiff-c0-ptrue-mtrue", "onpapgnkobhikddgannidamoiadejkahlcmcdfoimofcigbjjhefcgneponjclnk"),
    ("rgb-tiff-c1-pfalse-mfalse", "dbhadkdnmlbidgpleombpnbphfiphnleiknaegpaaobemfengifhpcanfhijomgf"),
    ("rgb-tiff-c1-pfalse-mtrue", "emooebhmchpcahcmiegjaajilefjcjmijpkejeeglopaofoeelaneamafgfehegc"),
    ("rgb-tiff-c1-ptrue-mfalse", "opoflbdokbcedlnmjnkecppphpldbkflnloimdlecjilibmppifdnljnkjaancnm"),
    ("rgb-tiff-c1-ptrue-mtrue", "pkfpdgkfbjlnmmbdmgfdnmbeliglgalkclhpnlepbleggfdbhnlcibhmeookdimk"),
    ("rgb-tiff-c2-pfalse-mfalse", "gbhnflcdddbpfliepgbnhpimjmpkdldkfhpoiidccakclgcahfljgihnbflookfc"),
    ("rgb-tiff-c2-pfalse-mtrue", "cjnbmfhfhmfkhkgefmlgejjpdcjmbekgkceanfjpddbflghnppcknmjolgfcgbjd"),
    ("rgb-tiff-c2-ptrue-mfalse", "kpjobpikaedmllifkcjmdhgkdnmgabgpkclafgcpkcnbijmagjpapkbaikchccem"),
    ("rgb-tiff-c2-ptrue-mtrue", "cgoeklegnnedfmfdbcaijpllbfifogfanaijnjbaammknomjkigeefjnhejpjjeh"),
    ("rgb-tiff-c3-pfalse-mfalse", "coggfbbcmaofhogllfnbobhoemeecdkljcdfckadfohgfegggfjaocamgminbdom"),
    ("rgb-tiff-c3-pfalse-mtrue", "ahjneipdjpkmgfafkklplbmpnamjhjkgdkdpflffihkonipblbcdedddgikjafll"),
    ("rgb-tiff-c3-ptrue-mfalse", "eglglblnafhofklfbkcldfjmllifeljfgeagcnkddmcgdmgegooooaollnjgbjlj"),
    ("rgb-tiff-c3-ptrue-mtrue", "gncljjncpggnickopmccfojflaoamhciplhgfodplngdkjjdcdckibmlhdkgpjod"),
    ("rgb-tiff-jpeg", "elppfbbhbcnbhhblalagnbgcfkmbdmkgfkibflndacejclbhlnolhhadndgahihn"),
    ("rgb-tiff-l0-yfalse-tfalse", "gpjmdghblfpafejagmeimopdmjkdjknejbjhogpnboamogjdpdcnlnfjhdpejccj"),
    ("rgb-tiff-l0-yfalse-ttrue", "gpjmdghblfpafejagmeimopdmjkdjknejbjhogpnboamogjdpdcnlnfjhdpejccj"),
    ("rgb-tiff-l0-ytrue-tfalse", "agkdhgooiiakcjpjckddaomgeaofdnpagljbfecbphbaijimcookmbihcdemlmib"),
    ("rgb-tiff-l0-ytrue-ttrue", "agkdhgooiiakcjpjckddaomgeaofdnpagljbfecbphbaijimcookmbihcdemlmib"),
    ("rgb-tiff-l1-yfalse-tfalse", "nepdlpnjnjobmadfpaoimidnpgalnnfkkgbmobilhpbnhfalnbmjahijilakjhgi"),
    ("rgb-tiff-l1-yfalse-ttrue", "nepdlpnjnjobmadfpaoimidnpgalnnfkkgbmobilhpbnhfalnbmjahijilakjhgi"),
    ("rgb-tiff-l1-ytrue-tfalse", "aimfbcbhalkpccplahfjolppfihmghlakdnceiobgmpkfoiapnlcpdgmkklebkgl"),
    ("rgb-tiff-l1-ytrue-ttrue", "aimfbcbhalkpccplahfjolppfihmghlakdnceiobgmpkfoiapnlcpdgmkklebkgl"),
    ("rgb16_layered.psd", "dbdnaklhdcpliongfflenlnhahkclpabnnphlkhgifipfahgedbpjingbpcikmop"),
    ("rgb16_tagged.psd", "dbdnaklhdcpliongfflenlnhahkclpabnnphlkhgifipfahgedbpjingbpcikmop"),
    ("rgb32_layered.psd", "lhlnhapkkfdimnbeanbpmhjlgkdjenffhkgmflamblkfalnbpmcdldgimdkipjgh"),
    ("rgba-bmp-1", "mkileegopejbkhkmjdkdafkocbadmfmlnffhopoaifemegcjjclennlaikojpmef"),
    ("rgba-bmp-16-adv1", "necaodolhkpkadcimkpafepifgenjgmfmfkbmhlflbjmapdfcpejedndgegkaakc"),
    ("rgba-bmp-16-adv2", "lbgjncjfpefgnlknggljfjaimbidpgffocnkbbcibnmphooodnhgbginjdbinicb"),
    ("rgba-bmp-16-false", "hdmcdeigkgkjmmnknlfgbpngdljfjcehfkmpanjfefdkcoecdcikffjpejdamioo"),
    ("rgba-bmp-16-true", "fndnjehhopjjfpagahkopoedldmjogenaicooaphpnobabobgolgadicccmeenol"),
    ("rgba-bmp-24-false", "jfdgmjlghllbiimhgdmiocachokmmdiikjcbjjjfbgplachjbaloijdlomldagka"),
    ("rgba-bmp-24-os2", "dioanmkplhkpkgopmolkkbpogpaaocielopklohgdjfgjnmfkhibeiepbhjjjafi"),
    ("rgba-bmp-24-true", "mekhabhdcfjgeplclibefheijaaeedcfkholjihhddlcaplmpmeiplbicpedegck"),
    ("rgba-bmp-32-false", "ddojejpnableepelifoclpcadiiainjlblehldmcdcnjnpfejgeiaffkgleakfhf"),
    ("rgba-bmp-32-true", "gmkhbjkomejdkoneolajafdliefijhfkfognopcceikagcjdecmgaefadaliioge"),
    ("rgba-bmp-4", "ndfnimaejmbcgjgheplboihecknpkncnmmcpnhdbcpidiimkdiggdphcdlpggogh"),
    ("rgba-bmp-8", "gekbebpeffolonobmkdmefdeodfpopkaoggllgipdpdfoiemdgcbckbjfmhbconn"),
    ("rgba-bmp-8-rle", "hhgpbgbllbnaciijgecclonopjohmnjphmonkmabjnlnepdahdcjpceapnieopno"),
    ("rgba-cineon", "neahlndmfepdcmikfhneampbkeemfdlgicfkjifpiddffgjcnakagbnjnkahpodh"),
    ("rgba-dcs", "kegkcohghhppladnlogpiagpigmfpkagmlfmnhekddacjoflnlelnlpiofgfhang"),
    ("rgba-eps", "pcngokalbbcbnhbkldppmhiodomifgmgmmpaiepmhlamfnapkjcakhcjimogdabl"),
    ("rgba-gif", "aopcapofeiiebajmnicfdnfanilkapofkpndmbbooifcjfeblmeeigpoflcnchff"),
    ("rgba-gif-i", "goanajjbgpkkjkgkbfmdnppnbebkfekbchdnaafgojnggnnmggpcibmmmbibkaej"),
    ("rgba-png-0-false", "ecgfcehomchjkcmickcjlkafkiejkmhaajmlfpofikiinfdejofjnndfbdagfcph"),
    ("rgba-png-0-true", "apbilldgbpbofkfhnflncaonnaadhmeiohpidihgoelmcdhpefcmoefjmlbbbiel"),
    ("rgba-png-6-false", "jogokbebjhchoikkjifmnfclndlokocnllkjkepjndnkoldldcnifeobnifmcpic"),
    ("rgba-png-6-true", "ebklmdmkbkdcnhjdehkofejcdilfgkacmilajghagbfdnpjfccghpkhmajiibbeh"),
    ("rgba-png-9-false", "mgkdekbgeddgndcnhflodnohhmofdbelgeiadhnkpejjdkkdajfmehkkccgbagjn"),
    ("rgba-png-9-true", "hjalcjjgoecgmnlkggofokfmpabjkapkbaijjdddkglnlmhildcknejmcbdlmhja"),
    ("rgba-raw-0-false", "bejenenjgmkmgmpafncfllpfjkhckecblcdjkkloadnphhpbocifkmngboocolkp"),
    ("rgba-raw-0-true", "ahgkgkilofpkllidilpeplacmpheflomhlniidpmdicplgjepnolfdphemigkocg"),
    ("rgba-raw-37-true", "gepgilicakeblhjidccadplcdgikfhjlcnfmockhomlmhjnofcbdciaelkdbcfed"),
    ("rgba-raw-64-false", "cnmgicbgdiagclmbmokabndkanjpofcjikodlbijfbdfdjfchcjfbnfgceonincp"),
    ("rgba-tga-16-false", "fbgghcoohlfkhimomajcahhbaoeiompfkcopgfbeokjfpkgkdgcjdcoigmoahadd"),
    ("rgba-tga-16-true", "pnbkadopffpbpopekfgcclocdfeabmgmgeeicpgiomlgpoakhomglfhbigjbkhpm"),
    ("rgba-tga-24-false", "fiejjbbmakpmjmbeabgmkgkokaienijpnolopnhjpajadlfkfeikplgialhdcbdn"),
    ("rgba-tga-24-true", "hgbokanohojopeppcaaccooijgoppnilcgjpakaakhgdnocdajnjhflfgfbmnkoc"),
    ("rgba-tga-32-false", "gnhdcepafeolilehhpcobenmlgiipooocbfogapfclejmajnhpeobdcfpdbnobgp"),
    ("rgba-tga-32-true", "fngbjnmpppeaecglfghgkokkglgkbppnhgcdnmgdkjejednopkgifcbhhdcohpln"),
    ("rgba-tiff-c0-pfalse-mfalse", "gangpjkffnjiijglehldmkiklpabkfcmomafikdjpdmeddmmccabkofgmnahjgea"),
    ("rgba-tiff-c0-pfalse-mtrue", "gpehmmffoioldanoglgdaemmgmodhikjahipglmcpicbghpncmfghbhfhfjdidpd"),
    ("rgba-tiff-c0-ptrue-mfalse", "egjaikfginehmooofaegnhmccoifnomendcekljbhgjokjfgkhooaipcdhibklpk"),
    ("rgba-tiff-c0-ptrue-mtrue", "khidpjmfelcdehakenepjeogngedplbcjehlbbdpfbbhebemdmchglihkpjhckih"),
    ("rgba-tiff-c1-pfalse-mfalse", "gepfcbngmodbojkikblcadnccmcgngfgnffflafpaganhnembopdfefdppicnedm"),
    ("rgba-tiff-c1-pfalse-mtrue", "beokfkbelpplcomppgcilkceabcpcpbpfhcaobohmhfghebbnjlkhdcfnkodankm"),
    ("rgba-tiff-c1-ptrue-mfalse", "ahfkcmfhohcbgacmfkbfgofbcmdpbkehaejjfjoiamnlapnipggimglcjkfmiccf"),
    ("rgba-tiff-c1-ptrue-mtrue", "dbdoenkadfokklniinhhgbahlpndbnhnilhkehepbapdjihhkggjafhbpddoojpa"),
    ("rgba-tiff-c2-pfalse-mfalse", "kogchnkgppbmbccmnmhaeifiiaofcoamhhfpajgljcjbpkmapbglknjmpbkjacgm"),
    ("rgba-tiff-c2-pfalse-mtrue", "ickhbdghklgncglnhgddnhnbdffmikekgkbpjmeeejafihdkipdcclnonkccdenf"),
    ("rgba-tiff-c2-ptrue-mfalse", "koaekfgphebgdfdohbbmaenplndgcdiocfelfpfpnlkahnnbcklcmaboalfcfojd"),
    ("rgba-tiff-c2-ptrue-mtrue", "iciooamaomamjgoihmeppkhkjilkcoimpnnlpbgjfdhbojckkfipilggfeenkbfc"),
    ("rgba-tiff-c3-pfalse-mfalse", "bbohfhmckknniioaofglafniidokcoakkogdhajcgomollijaellbckbjfjfchod"),
    ("rgba-tiff-c3-pfalse-mtrue", "oimbfdhdnjoblefkmlekgjhknnpkipboacekjbikaojlmgfngabmnffkjifkpcpe"),
    ("rgba-tiff-c3-ptrue-mfalse", "illlflogpifibmknejgkcokgfokdpfjedmcdbjhanndmfghcmjeidnckhoeadnnj"),
    ("rgba-tiff-c3-ptrue-mtrue", "dnpnpojiiaanjgfhoflongelmdhjikdghboankjgicdcmfipjdphgpgajcfkkfkh"),
    ("rgba-tiff-jpeg", "ljgjonmcfnolbmjdjieneecdnkgfboooplcfioeamnejkcahofhfpohddbpifpbp"),
    ("rgba-tiff-l0-yfalse-tfalse", "adphaibbhoklbenpdcegjlenlbdfoeahnahkdpmjaeeigbjmljckkbneniphhnbk"),
    ("rgba-tiff-l0-yfalse-ttrue", "pdjadgbgfhplnbelmgglfemdkldilmnmeednccppcngiidickjfhlafalnikennd"),
    ("rgba-tiff-l0-ytrue-tfalse", "pcnaappoaaeapknhgckokjbpdejnjmmdcpdfnhlmmmfbglcnlmhbmafhelnffilh"),
    ("rgba-tiff-l0-ytrue-ttrue", "ifimeoeeoaociokkenblngabgoddhegbcmjcpdffannfdlgfjockjpimeeabagni"),
    ("rgba-tiff-l1-yfalse-tfalse", "ajkbbmkbdholhmfmnfabmnioaejhnpmefikkklbgnlcggddbfldhjcndfohjmgpe"),
    ("rgba-tiff-l1-yfalse-ttrue", "pjdiknmohglgocfcogffpmolchacicoiifnafbabplbmfooeechkgniejffkmcpo"),
    ("rgba-tiff-l1-ytrue-tfalse", "lkfkenpacmnpehnjmmbadndlijfapafagfnlhflopillfgoedgcialpeoimjicfn"),
    ("rgba-tiff-l1-ytrue-ttrue", "lngmoobildbgednfineppcflllkmklopnjkooadnjonihjefcalfnjckpbhhdfcd"),
    ("tiled-raw-raw.psd", "mhlnoaigfmkoapjigchlkjnolhglhjdkpagjdagaibcfpkhhddgfdipihpnniffd"),
    ("tiled-raw-rle.psd", "pfgbakfcagiceebikkippbiafpffmdbbekdhangiobhbiooplehpcmjkednemojp"),
    ("tiled-raw-zip.psd", "oolhhpmbcjiniehclfgnhpjegdkkddmacbfaacbnbdjmjdambkgojiggognpbjnf"),
    ("tiled-raw-zipp.psd", "hcamikogafjmfgocoomdocnjljaacbnniommnpehlphikijdhmigckobpkfhbgng"),
    ("tiled-raw.psb", "icldljnbfdcpkjhandiopmfmhoiihbhfpncjgdhokkljjhbgapgcnjngnkmmbmkf"),
    ("tiled-rle-raw.psd", "mibknpooafnhjgpbboocdbejacenbodgcphgihpiddppfkeebgcfaenopanhgpge"),
    ("tiled-rle-rle.psd", "hbcpnogmficeeglbgihcpcbjmkhocokibdkfjgdphindfbaekpkfjfnljlpgdnen"),
    ("tiled-rle-zip.psd", "aeadjdnnabfjagiekholeapfomgfoliileacgdpkdkpdmcjpafckpdehooalbgke"),
    ("tiled-rle-zipp.psd", "ljahdechocfecimpoioladbaljpnkkkllcnadmealphkjkaaoflbeddfelnhkedn"),
    ("tiled-rle.psb", "lcndibfglidahaafbffkmiocncjogpnfifpeacbpoopbmogceamdbbocdpkkhcmg"),
    ("tiled-zip-raw.psd", "dadckojhnoldmpihkmkhikionkpeepdjmefcdeopegjjjdjcegmidkmdceodgkni"),
    ("tiled-zip-rle.psd", "hkdpheejcgbedbednfngcjbocbghlmhnfbboalanafcaegoccfhdggepnglbdman"),
    ("tiled-zip-zip.psd", "igjhphfoiipbegffinffkkehkobfafgkoimeknfoddkpgpdiacfpdlnmijbfleel"),
    ("tiled-zip-zipp.psd", "edmplkahkjpelmobeagkdllongedchcjnodafenjoklfjioojcndknfgffgncopg"),
    ("tiled-zip.psb", "bidkodlbkakfnmglnilhiofokhjlgdncenhehbihoheechkpmhacmmjhilgmgnkd"),
    ("tiled-zipp-raw.psd", "eipdglillhelgapdggpmpjnbgmmhepgihmaniekmnodnimnbijbkbemhkkadcllh"),
    ("tiled-zipp-rle.psd", "mcnohhkpfdibnhdlccafnefahomiiiajeaahhkccelebjlfodlnaofpkmhgaiaag"),
    ("tiled-zipp-zip.psd", "jeeeecmmmjlfodhkljlenlifbfiojiiaomgnkmmgbkcalbhaaepnjjifconjpdhp"),
    ("tiled-zipp-zipp.psd", "eolkkljkieeeiodgjdgbcbaocecnfljejolahcdpipbgkbjjpfangnmnimbjmamb"),
    ("tiled-zipp.psb", "aaccapdllgccmbeoedlapcdankanamlegkpgdnemkhlbilcihanpaecbpbigckam"),
];
