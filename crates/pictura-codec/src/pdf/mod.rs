//! Photoshop PDF writer (`docs/10-workflow-io/save-and-save-as.md`, *Save in
//! Photoshop PDF format*; OpenSpec `interop/pdf-export`). One page holding the
//! document composite as one image, its top type layers drawn over it as live
//! text; see the change's `design.md` for the decisions (D1–D13).

mod ccitt;
mod crypt;
mod image;
pub mod import;
mod joboptions;
mod linearize;
mod object;
mod options;
mod presets;
mod read;
mod resample;
mod text;
mod xmp;

#[cfg(test)]
mod tests;

use std::io::Write as _;

use pictura_core::{Document, MarkerKind, PixelBuffer};

pub use joboptions::{parse_joboptions, same_preset, write_joboptions};
pub use options::{
    Changes, ColorConversion, Compatibility, Compression, Destination, Downsample, Encryption,
    OutputIntent, PdfOptions, Printing, ProfileInclusion, Quality, Security, Standard, FIELDS,
};
pub use presets::{builtin_preset, BUILTIN_PRESETS};
pub use read::{embedded_psd, photoshop_psd};
pub use text::{LiveText, PdfFont, PdfGlyph, PdfText, TextPolicy};

use crate::error::PsdError;
use crypt::{Encryptor, Entropy};
use image::{Raster, Space};
use object::{Body, Dict, Obj, Objects};

/// What the JPEG edge is asked to encode: interleaved 8-bit samples.
pub struct JpegRequest<'a> {
    pub samples: &'a [u8],
    pub width: u32,
    pub height: u32,
    /// 1 (gray), 3, or 4 (CMYK, 0 = no ink).
    pub components: u8,
    /// 0–100.
    pub quality: u8,
}

/// The writer's environment: the JPEG encoder (the Qt boundary in the app),
/// the clock, the randomness, and the title.
pub struct PdfEnv<'a> {
    pub jpeg: &'a dyn Fn(&JpegRequest) -> Option<Vec<u8>>,
    /// Unix seconds for the dates.
    pub now: i64,
    /// Seeds the file ID and the encryption salts and IVs.
    pub entropy: [u8; 32],
    /// The document title (the file name without its extension).
    pub title: String,
    /// Type layers drawn as live text over a page image that omits them; the
    /// editing payload and thumbnail still take the full composite.
    pub live_text: Option<LiveText<'a>>,
}

const PRODUCER: &str = "Kooka Pictura";

/// The objects of one file and the roles linearization needs.
pub(crate) struct Built {
    pub objects: Objects,
    pub catalog: u32,
    pub info: u32,
    pub page: u32,
    pub content: u32,
    /// The page object and everything only it references, page first.
    pub page_objects: Vec<u32>,
    pub encrypt: Option<u32>,
    pub encryptor: Option<Encryptor>,
    pub id: [u8; 16],
    pub minor: u8,
}

fn flate(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("writing to a Vec");
    encoder.finish().expect("writing to a Vec")
}

/// `icc` as embedded at header minor version `minor`: PDF 1.4 and older
/// predate ICC v4, so the profile is re-serialized as v2 there.
fn embedded_icc(icc: &[u8], minor: u8) -> Vec<u8> {
    match pictura_color::Profile::from_icc(icc) {
        Ok(profile) if minor <= 4 && icc.get(8).is_some_and(|&v| v >= 4) => profile.to_icc_v2(),
        _ => icc.to_vec(),
    }
}

fn icc_stream(objects: &mut Objects, n: u8, icc: &[u8], minor: u8) -> u32 {
    let icc = &embedded_icc(icc, minor);
    let alternate = match n {
        1 => "DeviceGray",
        4 => "DeviceCMYK",
        _ => "DeviceRGB",
    };
    objects.add(Body::Stream(
        Dict::new()
            .with("N", Obj::Int(i64::from(n)))
            .with("Alternate", Obj::name(alternate))
            .with("Filter", Obj::name("FlateDecode")),
        flate(icc),
    ))
}

fn color_space(objects: &mut Objects, owned: &mut Vec<u32>, space: &Space, minor: u8) -> Obj {
    match space {
        Space::Gray => Obj::name("DeviceGray"),
        Space::Rgb => Obj::name("DeviceRGB"),
        Space::Cmyk => Obj::name("DeviceCMYK"),
        Space::Lab => Obj::Array(vec![
            Obj::name("Lab"),
            Dict::new()
                .with("WhitePoint", Obj::reals(&[0.9642, 1.0, 0.8249]))
                .with("Range", Obj::reals(&[-128.0, 127.0, -128.0, 127.0]))
                .into(),
        ]),
        Space::Indexed(palette) => Obj::Array(vec![
            Obj::name("Indexed"),
            Obj::name("DeviceRGB"),
            Obj::Int(255),
            Obj::Str(palette.clone()),
        ]),
        Space::Icc(n, icc) => {
            let stream = icc_stream(objects, *n, icc, minor);
            owned.push(stream);
            Obj::Array(vec![Obj::name("ICCBased"), Obj::Ref(stream)])
        }
    }
}

/// The image XObject's dictionary and data for `raster`.
fn image_stream(
    raster: &Raster,
    options: &PdfOptions,
    env: &PdfEnv,
) -> Result<(Dict, Vec<u8>), PsdError> {
    let mut dict = Dict::typed("XObject")
        .with("Subtype", Obj::name("Image"))
        .with("Width", Obj::Int(i64::from(raster.width)))
        .with("Height", Obj::Int(i64::from(raster.height)))
        .with("BitsPerComponent", Obj::Int(i64::from(raster.bpc)));
    let data = match (&raster.bits, options.compression) {
        (_, Compression::None) => raster.samples.clone(),
        (Some(bits), _) => {
            dict.set("Filter", Obj::name("CCITTFaxDecode"));
            dict.set(
                "DecodeParms",
                Dict::new()
                    .with("K", Obj::Int(-1))
                    .with("Columns", Obj::Int(i64::from(raster.width)))
                    .with("Rows", Obj::Int(i64::from(raster.height)))
                    .into(),
            );
            ccitt::encode_g4(bits, raster.width as usize)
        }
        (None, Compression::Jpeg | Compression::Jpeg2000) if raster.jpeg_capable() => {
            let components = raster.space.components() as u8;
            let bytes = (env.jpeg)(&JpegRequest {
                samples: &raster.samples,
                width: raster.width,
                height: raster.height,
                components,
                quality: options.quality.encoder_quality(),
            })
            .ok_or_else(|| PsdError::Unsupported("JPEG encoding failed".into()))?;
            dict.set("Filter", Obj::name("DCTDecode"));
            if components == 4 {
                // A CMYK JPEG carries Adobe's inverted samples.
                dict.set(
                    "Decode",
                    Obj::reals(&[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]),
                );
            }
            bytes
        }
        _ => {
            dict.set("Filter", Obj::name("FlateDecode"));
            flate(&raster.samples)
        }
    };
    Ok((dict, data))
}

/// The document's resolution in ppi (72 when unknown).
fn document_ppi(doc: &Document) -> f64 {
    crate::image_resources::document_resolution(doc)
        .map(|r| r.ppi)
        .filter(|p| p.is_finite() && *p > 0.0)
        .unwrap_or(72.0)
}

/// The PSD payload Preserve Photoshop Editing Capabilities embeds, honouring
/// the Layers, Alpha Channels, and Notes options (design D6).
fn editing_payload(
    doc: &Document,
    composite: &PixelBuffer,
    options: &PdfOptions,
) -> Result<Vec<u8>, PsdError> {
    let mut payload = doc.clone();
    if !options.layers {
        let color = doc.mode.color_channels() as usize;
        let plane = composite.pixel_count();
        let mut planes = composite.data[..color * plane].to_vec();
        if composite.channels as usize > color {
            let alpha = &composite.data[color * plane..(color + 1) * plane];
            for c in 0..color {
                for (v, &a) in planes[c * plane..(c + 1) * plane].iter_mut().zip(alpha) {
                    *v = ((u32::from(*v) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255)
                        as u8;
                }
            }
        }
        payload.layers.clear();
        payload.global_layer_mask.clear();
        payload.layer_section_extra.clear();
        payload.merged_composite_present = true;
        payload.composite = PixelBuffer {
            width: composite.width,
            height: composite.height,
            channels: color as u8,
            data: planes.into(),
        };
    }
    if !options.alpha_channels {
        payload.channels.clear();
    }
    if !options.notes {
        payload.annotations.clear(MarkerKind::Note);
    }
    crate::write::write_psd(&payload)
}

/// Serialize `doc` (the save view) and its rendered `composite` as a Photoshop
/// PDF. Fails, writing nothing, for Multichannel, a failed JPEG encode, or an
/// unwritable document.
pub fn write_pdf(
    doc: &Document,
    composite: &PixelBuffer,
    options: &PdfOptions,
    env: &PdfEnv,
) -> Result<Vec<u8>, PsdError> {
    let mut options = options.clone();
    options.normalize();
    let built = build(doc, composite, &options, env)?;
    Ok(if options.fast_web_view {
        linearize::write(&built)
    } else {
        write_plain(&built)
    })
}

fn build(
    doc: &Document,
    composite: &PixelBuffer,
    options: &PdfOptions,
    env: &PdfEnv,
) -> Result<Built, PsdError> {
    let ppi = document_ppi(doc);
    let live = env.live_text.as_ref().filter(|t| !t.texts.is_empty());
    let raster = match live {
        Some(live) => {
            let background = &live.background;
            if (background.width, background.height) != (composite.width, composite.height) {
                return Err(PsdError::Invalid(
                    "the live-text background does not match the composite".into(),
                ));
            }
            // The retained source planes still hold the type's pixels; the
            // background is encoded from its own samples.
            let without_type = Document {
                source_planes: None,
                ..doc.clone()
            };
            image::build_raster(&without_type, background, options, ppi)?
        }
        None => image::build_raster(doc, composite, options, ppi)?,
    };
    let entropy = Entropy::new(env.entropy);
    let id: [u8; 16] = entropy.bytes();
    let points = |px: u32| f64::from(px) * 72.0 / ppi;
    let (page_w, page_h) = (points(doc.width), points(doc.height));
    let standard = options.standard;
    let date = xmp::pdf_date(env.now);
    let minor = options.compatibility.minor();

    let mut objects = Objects::default();
    let catalog = objects.reserve();
    let pages = objects.reserve();
    let page = objects.reserve();
    let mut page_objects = vec![page];

    let content = objects.reserve();
    page_objects.push(content);
    let mut operators = format!(
        "q {} 0 0 {} 0 0 cm /Im0 Do Q\n",
        object::real(page_w),
        object::real(page_h)
    );

    let image = objects.reserve();
    page_objects.push(image);
    let (mut image_dict, image_data) = image_stream(&raster, options, env)?;
    let space = color_space(&mut objects, &mut page_objects, &raster.space, minor);
    image_dict.set("ColorSpace", space.clone());
    let mut resources = Dict::new();
    if let Some(live) = live {
        let page = text::Page {
            points: 72.0 / ppi,
            height: page_h,
        };
        let mut fonts = Dict::new();
        let mut states = Dict::new();
        let mut named_space = false;
        let mut embedded = Vec::new();
        for font in &live.fonts {
            embedded.push(text::embed_font(&mut objects, &mut page_objects, font));
        }
        for (i, run) in live.texts.iter().enumerate() {
            let Some(font) = embedded.get(run.font) else {
                continue;
            };
            let font_name = format!("F{}", run.font);
            fonts.set(&font_name, Obj::Ref(font.font));
            let fill = image::text_fill(doc, options, &raster, run.color)?;
            named_space |= matches!(fill, text::Fill::Space(_));
            let alpha = (run.opacity < 255).then(|| format!("GS{i}"));
            if let Some(gs) = &alpha {
                let a = f64::from(run.opacity) / 255.0;
                states.set(
                    gs,
                    Dict::typed("ExtGState")
                        .with("ca", Obj::Real(a))
                        .with("CA", Obj::Real(a))
                        .into(),
                );
            }
            operators.push_str(&text::operators(
                run,
                &font_name,
                font,
                &fill,
                alpha.as_deref(),
                &page,
            ));
        }
        resources.set("Font", fonts.into());
        if !states.0.is_empty() {
            resources.set("ExtGState", states.into());
        }
        if named_space {
            resources.set("ColorSpace", Dict::new().with("CS0", space).into());
        }
    }
    objects.set(
        content,
        Body::Stream(
            Dict::new().with("Filter", Obj::name("FlateDecode")),
            flate(operators.as_bytes()),
        ),
    );
    if let Some(alpha) = &raster.alpha {
        let smask = objects.add(Body::Stream(
            Dict::typed("XObject")
                .with("Subtype", Obj::name("Image"))
                .with("Width", Obj::Int(i64::from(raster.width)))
                .with("Height", Obj::Int(i64::from(raster.height)))
                .with("ColorSpace", Obj::name("DeviceGray"))
                .with("BitsPerComponent", Obj::Int(8))
                .with("Filter", Obj::name("FlateDecode")),
            flate(alpha),
        ));
        page_objects.push(smask);
        image_dict.set("SMask", Obj::Ref(smask));
    }
    objects.set(image, Body::Stream(image_dict, image_data));

    let media_box = Obj::reals(&[0.0, 0.0, page_w, page_h]);
    let proc_image = match raster.space {
        Space::Indexed(_) => "ImageI",
        Space::Gray => "ImageB",
        _ => "ImageC",
    };
    let mut proc_set = vec![Obj::name("PDF"), Obj::name(proc_image)];
    if live.is_some() {
        proc_set.push(Obj::name("Text"));
    }
    resources.set("XObject", Dict::new().with("Im0", Obj::Ref(image)).into());
    resources.set("ProcSet", Obj::Array(proc_set));
    let mut page_dict = Dict::typed("Page")
        .with("Parent", Obj::Ref(pages))
        .with("MediaBox", media_box.clone())
        .with("Resources", resources.into())
        .with("Contents", Obj::Ref(content));
    if standard.is_pdfx() {
        page_dict.set("TrimBox", media_box);
    }
    if options.embed_thumbnails {
        let (tw, th, gray, samples) = image::thumbnail(doc, composite);
        let thumb = objects.add(Body::Stream(
            Dict::new()
                .with("Width", Obj::Int(i64::from(tw)))
                .with("Height", Obj::Int(i64::from(th)))
                .with(
                    "ColorSpace",
                    Obj::name(if gray { "DeviceGray" } else { "DeviceRGB" }),
                )
                .with("BitsPerComponent", Obj::Int(8))
                .with("Filter", Obj::name("FlateDecode")),
            flate(&samples),
        ));
        page_objects.push(thumb);
        page_dict.set("Thumb", Obj::Ref(thumb));
    }
    // ponytail: PDF/X forbids annotations on the trimmed page and PDF/A-1 asks
    // for appearance streams, so notes are written only without a standard.
    let notes = doc.annotations.markers(MarkerKind::Note);
    if options.notes && standard == Standard::None && !notes.is_empty() {
        let mut annots = Vec::new();
        for note in notes {
            let x = f64::from(note.x) * 72.0 / ppi;
            let y = page_h - f64::from(note.y) * 72.0 / ppi;
            let annot = objects.add(Body::Obj(
                Dict::typed("Annot")
                    .with("Subtype", Obj::name("Text"))
                    .with("Rect", Obj::reals(&[x, y - 20.0, x + 20.0, y]))
                    .with("Contents", Obj::text(&note.text))
                    .with("Name", Obj::name("Note"))
                    .with("F", Obj::Int(28))
                    .with("Open", Obj::Bool(false))
                    .into(),
            ));
            page_objects.push(annot);
            annots.push(Obj::Ref(annot));
        }
        page_dict.set("Annots", Obj::Array(annots));
    }
    if options.preserve_editing {
        // ponytail: Pictura's own payload (design D6), not Adobe's layout.
        let psd = objects.add(Body::Stream(
            Dict::new()
                .with("Subtype", Obj::name("PicturaPSD"))
                .with("Filter", Obj::name("FlateDecode")),
            flate(&editing_payload(doc, composite, options)?),
        ));
        page_objects.push(psd);
        page_dict.set("LastModified", Obj::text(&date));
        page_dict.set(
            "PieceInfo",
            Dict::new()
                .with(
                    "Pictura",
                    Dict::new()
                        .with("LastModified", Obj::text(&date))
                        .with("Private", Dict::new().with("PSD", Obj::Ref(psd)).into())
                        .into(),
                )
                .into(),
        );
    }
    objects.set(page, Body::Obj(page_dict.into()));
    objects.set(
        pages,
        Body::Obj(
            Dict::typed("Pages")
                .with("Kids", Obj::Array(vec![Obj::Ref(page)]))
                .with("Count", Obj::Int(1))
                .into(),
        ),
    );

    let document_id: String = id.iter().map(|b| format!("{b:02x}")).collect();
    let mut info_dict = Dict::new()
        .with("Title", Obj::text(&env.title))
        .with("Producer", Obj::text(PRODUCER))
        .with("Creator", Obj::text(PRODUCER))
        .with("CreationDate", Obj::text(&date))
        .with("ModDate", Obj::text(&date));
    if standard.is_pdfx() {
        let version = match standard {
            Standard::X1a2001 => "PDF/X-1:2001",
            Standard::X32002 => "PDF/X-3:2002",
            _ => "PDF/X-4",
        };
        info_dict.set("GTS_PDFXVersion", Obj::text(version));
        if standard == Standard::X1a2001 {
            info_dict.set("GTS_PDFXConformance", Obj::text("PDF/X-1a:2001"));
        }
        info_dict.set("Trapped", Obj::name("False"));
    }
    let info = objects.add(Body::Obj(info_dict.into()));

    let mut catalog_dict = Dict::typed("Catalog").with("Pages", Obj::Ref(pages));
    if options.compatibility >= Compatibility::Acrobat5 {
        let packet = xmp::packet(
            &xmp::DocInfo {
                title: &env.title,
                producer: PRODUCER,
                creator: PRODUCER,
                now: env.now,
                document_id: &document_id,
            },
            standard,
        );
        let metadata = objects.add(Body::Stream(
            Dict::typed("Metadata").with("Subtype", Obj::name("XML")),
            packet.into_bytes(),
        ));
        if options.security.plaintext_metadata && options.compatibility >= Compatibility::Acrobat7 {
            objects.mark_plain(metadata);
        }
        catalog_dict.set("Metadata", Obj::Ref(metadata));
    }
    if standard != Standard::None {
        let pdfa = standard == Standard::A1b2005;
        let cmyk_output = raster.space.components() == 4;
        let (n, icc) = if !pdfa || cmyk_output {
            (4, Destination::CoatedCmyk.icc())
        } else {
            (3, Destination::Srgb.icc())
        };
        let profile = icc_stream(&mut objects, n, &icc, minor);
        let intent = &options.output_intent;
        let mut intent_dict = Dict::typed("OutputIntent")
            .with("S", Obj::name(if pdfa { "GTS_PDFA1" } else { "GTS_PDFX" }))
            .with("DestOutputProfile", Obj::Ref(profile));
        if pdfa {
            let name = if cmyk_output {
                Destination::CoatedCmyk.label()
            } else {
                Destination::Srgb.label()
            };
            intent_dict.set("OutputConditionIdentifier", Obj::text(name));
            intent_dict.set("Info", Obj::text(name));
        } else {
            intent_dict.set(
                "OutputConditionIdentifier",
                Obj::text(&intent.condition_identifier),
            );
            intent_dict.set("OutputCondition", Obj::text(&intent.condition));
            intent_dict.set("RegistryName", Obj::text(&intent.registry_name));
            intent_dict.set("Info", Obj::text(&intent.profile_name));
        }
        catalog_dict.set("OutputIntents", Obj::Array(vec![intent_dict.into()]));
    }

    let encryptor = (options.security.encrypts() && standard == Standard::None).then(|| {
        Encryptor::new(
            &options.security,
            options.compatibility.encryption(),
            &id,
            &entropy,
        )
    });
    let encrypt = encryptor.as_ref().map(|e| {
        let num = objects.add(Body::Obj(e.dict.clone().into()));
        objects.mark_plain(num);
        num
    });
    // AES-256 (revision 6) is Adobe extension level 8 to PDF 1.7.
    if options.compatibility == Compatibility::AcrobatX && encryptor.is_some() {
        catalog_dict.set(
            "Extensions",
            Dict::new()
                .with(
                    "ADBE",
                    Dict::new()
                        .with("BaseVersion", Obj::name("1.7"))
                        .with("ExtensionLevel", Obj::Int(8))
                        .into(),
                )
                .into(),
        );
    }
    objects.set(catalog, Body::Obj(catalog_dict.into()));
    Ok(Built {
        objects,
        catalog,
        info,
        page,
        content,
        page_objects,
        encrypt,
        encryptor,
        id,
        minor,
    })
}

/// `%PDF-1.x` plus the binary comment marking the file as binary.
pub(crate) fn header(minor: u8) -> Vec<u8> {
    let mut out = format!("%PDF-1.{minor}\n").into_bytes();
    out.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");
    out
}

/// The trailer's `/ID`, `/Root`, `/Info`, and `/Encrypt` entries.
pub(crate) fn trailer_entries(built: &Built, map: &dyn Fn(u32) -> u32) -> Dict {
    let mut dict = Dict::new()
        .with("Root", Obj::Ref(map(built.catalog)))
        .with("Info", Obj::Ref(map(built.info)))
        .with(
            "ID",
            Obj::Array(vec![
                Obj::Str(built.id.to_vec()),
                Obj::Str(built.id.to_vec()),
            ]),
        );
    if let Some(encrypt) = built.encrypt {
        dict.set("Encrypt", Obj::Ref(map(encrypt)));
    }
    dict
}

fn write_plain(built: &Built) -> Vec<u8> {
    let mut out = header(built.minor);
    let cipher = built
        .encryptor
        .as_ref()
        .map(|e| move |num: u32, data: &[u8]| e.encrypt(num, data));
    let cipher_ref: Option<object::Cipher> = cipher.as_ref().map(|c| c as object::Cipher);
    let mut offsets = Vec::new();
    for num in 1..=built.objects.count() {
        offsets.push(out.len());
        out.extend(built.objects.serialize(num, cipher_ref));
    }
    let xref = out.len();
    out.extend(object::xref_section(1, &offsets));
    let trailer =
        trailer_entries(built, &|n| n).with("Size", Obj::Int(i64::from(built.objects.count()) + 1));
    out.extend_from_slice(b"trailer\n");
    object::write_obj(&mut out, &trailer.into(), 0, None);
    out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}
