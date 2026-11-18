use super::*;
use crate::cxxqt_object::{helpers_composite, picture};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use ffi::PrintRaster;
use pictura_codec::pdf::{write_print_pdf, PrintImage, PrintPage};
use pictura_color::proof::{print_gamut_mask, print_gamut_mask_source, HardProof};
use pictura_color::{convert_space, Intent, Profile, ProofPreview, ProofSetup, SpaceTransform};
use pictura_core::{ColorMode, Document, SampleRow, SourcePlanes};
use serde_json::{json, Value};

fn empty_raster(message: impl AsRef<str>) -> PrintRaster {
    PrintRaster {
        width: 0,
        height: 0,
        components: 0,
        samples: Vec::new(),
        profile: Vec::new(),
        error: QString::from(message.as_ref()),
    }
}

fn rgb_raster(width: u32, height: u32, samples: Vec<u8>) -> PrintRaster {
    PrintRaster {
        width,
        height,
        components: 4,
        samples,
        profile: Vec::new(),
        error: QString::default(),
    }
}

pub(super) fn print_document_description(view: &PictureView) -> QString {
    let Some(doc) = &view.rust().session.doc else {
        return QString::default();
    };
    let metadata = pictura_codec::read_metadata(doc);
    let value = metadata
        .iptc
        .text(2, 120)
        .filter(|value| !value.is_empty())
        .or_else(|| pictura_codec::xmp_properties(doc).description)
        .unwrap_or_default();
    QString::from(value)
}

pub(super) fn print_source_preview(view: &PictureView, max_side: u32) -> PrintRaster {
    let view = view.rust();
    let Some(doc) = &view.session.doc else {
        return empty_raster("There is no document to print.");
    };
    let buffer = picture::preview_picture(doc, view, max_side.max(1));
    let buffer = pictura_codec::buffer_to_srgb(doc, &buffer);
    rgb_raster(
        buffer.width,
        buffer.height,
        helpers_composite::buffer_to_rgba_bytes(&buffer),
    )
}

pub(super) fn print_output_profile_info(icc: &[u8]) -> QString {
    let result = Profile::from_icc(icc).ok().and_then(|profile| {
        SpaceTransform::new(
            &Profile::srgb(),
            &profile,
            Intent::RelativeColorimetric,
            true,
        )
        .ok()?;
        Some(json!({ "name": profile.description()?, "components": profile.channels()? }))
    });
    result.map_or_else(QString::default, |value| QString::from(value.to_string()))
}

pub(super) fn print_builtin_profile(device: i32) -> Vec<u8> {
    match device {
        0..=4 => proof_device_icc(device),
        _ => Vec::new(),
    }
}

fn proof_setup(bytes: &[u8], settings: &Value) -> Result<ProofSetup, String> {
    ProofSetup::from_device_icc(
        bytes,
        intent_from_number(settings["intent"].as_i64().unwrap_or(1)),
        settings["bpc"].as_bool().unwrap_or(true),
        settings["paper"].as_bool().unwrap_or(false),
        settings["black"].as_bool().unwrap_or(false),
    )
    .map_err(|_| "The print proof profile is missing or invalid.".into())
}

pub(super) fn print_paper_color(icc: &[u8]) -> Vec<u8> {
    let Ok(profile) = Profile::from_icc(icc) else {
        return Vec::new();
    };
    let white: &[u8] = match profile.channels() {
        Some(1) => &[255],
        Some(3) => &[255; 3],
        Some(4) => &[0; 4],
        _ => return Vec::new(),
    };
    convert_space(
        &profile,
        &Profile::srgb(),
        white,
        Intent::AbsoluteColorimetric,
        false,
    )
    .unwrap_or_default()
}

fn output_options(settings: &Value) -> (Intent, bool) {
    (
        intent_from_number(settings["intent"].as_i64().unwrap_or(1)),
        settings["bpc"].as_bool().unwrap_or(true),
    )
}

pub(super) fn print_device_color(rgb: &[u8], output: &[u8], settings: &QString) -> Vec<u8> {
    if output.is_empty() {
        return rgb.to_vec();
    }
    let Ok(profile) = Profile::from_icc(output) else {
        return Vec::new();
    };
    let (intent, bpc) = output_options(&parse(settings));
    convert_space(&Profile::srgb(), &profile, rgb, intent, bpc).unwrap_or_default()
}

pub(super) fn print_preview_pixels(
    rgba: &[u8],
    output: &[u8],
    proof: &[u8],
    settings: &QString,
) -> Vec<u8> {
    let settings = parse(settings);
    if !rgba.len().is_multiple_of(4) {
        return Vec::new();
    }
    let Ok(output) = Profile::from_icc(output) else {
        return Vec::new();
    };
    let (intent, bpc) = output_options(&settings);
    let display = Profile::srgb();
    let mut result = rgba.to_vec();
    let source: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    if settings["hardProof"].as_bool() == Some(true) {
        let Ok(proof) = proof_setup(proof, &settings["proof"]) else {
            return Vec::new();
        };
        let Ok(transform) = HardProof::new(&display, &output, intent, bpc, &proof) else {
            return Vec::new();
        };
        let Ok(device) = transform.convert(&source) else {
            return Vec::new();
        };
        let Ok(preview) = convert_space(
            &output,
            &display,
            &device,
            Intent::RelativeColorimetric,
            false,
        ) else {
            return Vec::new();
        };
        let (pixels, _) = result.as_chunks_mut::<4>();
        for (rgba, rgb) in pixels.iter_mut().zip(preview.as_chunks::<3>().0) {
            rgba[..3].copy_from_slice(rgb);
        }
        if settings["gamut"].as_bool() == Some(true) {
            let setup = ProofSetup {
                device: output,
                intent,
                bpc,
                simulate_paper: false,
                simulate_black: false,
            };
            let Ok(mask) = print_gamut_mask(&source, &setup) else {
                return Vec::new();
            };
            for (pixel, warning) in pixels.iter_mut().zip(mask) {
                if warning {
                    pixel[..3].copy_from_slice(&[230, 0, 230]);
                }
            }
        }
        return result;
    }
    let setup = ProofSetup {
        device: output,
        intent,
        bpc,
        simulate_paper: false,
        simulate_black: false,
    };
    let Ok(transform) = ProofPreview::new(&setup) else {
        return Vec::new();
    };
    transform.apply(&mut result);
    if settings["gamut"].as_bool() == Some(true) {
        let Ok(mask) = print_gamut_mask(&source, &setup) else {
            return Vec::new();
        };
        for (pixel, warning) in result.as_chunks_mut::<4>().0.iter_mut().zip(mask) {
            if warning {
                pixel[..3].copy_from_slice(&[230, 0, 230]);
            }
        }
    }
    result
}

enum Conversion {
    Unmanaged,
    Managed(SpaceTransform),
    Proof(HardProof),
}

struct PrintConversion {
    output: Profile,
    conversion: Conversion,
    background: Vec<u8>,
    source_components: usize,
    tagged: bool,
}

impl PrintConversion {
    fn new(
        source: &Profile,
        output: &[u8],
        proof: &[u8],
        settings: &Value,
    ) -> Result<Self, String> {
        let mut background = [255; 3];
        if let Some(rgb) = settings["background"].as_array() {
            for (channel, value) in background.iter_mut().zip(rgb) {
                *channel = value.as_f64().unwrap_or(255.0).round() as u8;
            }
        }
        let background = convert_space(
            &Profile::srgb(),
            source,
            &background,
            Intent::RelativeColorimetric,
            true,
        )
        .map_err(|error| error.to_string())?;
        let source_components = background.len();
        if settings["managed"].as_bool() != Some(true) {
            let output = Profile::from_icc(&source.to_icc()).map_err(|error| error.to_string())?;
            return Ok(Self {
                output,
                conversion: Conversion::Unmanaged,
                background,
                source_components,
                tagged: !source.is_srgb(),
            });
        }
        let output = Profile::from_icc(output)
            .map_err(|_| "Choose an available printer ICC profile.".to_owned())?;
        let (intent, bpc) = output_options(settings);
        let conversion = if settings["hardProof"].as_bool() == Some(true) {
            let proof = proof_setup(proof, &settings["proof"])?;
            Conversion::Proof(
                HardProof::new(source, &output, intent, bpc, &proof)
                    .map_err(|error| error.to_string())?,
            )
        } else {
            Conversion::Managed(
                SpaceTransform::new(source, &output, intent, bpc)
                    .map_err(|error| error.to_string())?,
            )
        };
        Ok(Self {
            output,
            conversion,
            background,
            source_components,
            tagged: true,
        })
    }

    fn flatten(&self, rgba: &[u8]) -> Vec<u8> {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| {
                let alpha = u32::from(pixel[3]);
                std::array::from_fn::<_, 3, _>(|channel| {
                    ((u32::from(pixel[channel]) * alpha
                        + u32::from(self.background[channel]) * (255 - alpha)
                        + 127)
                        / 255) as u8
                })
            })
            .collect()
    }

    fn append(&self, rgba: &[u8], output: &mut Vec<u8>) {
        let source = self.flatten(rgba);
        self.append_pixels(&source, output);
    }

    fn append_pixels(&self, source: &[u8], output: &mut Vec<u8>) {
        match &self.conversion {
            Conversion::Unmanaged => output.extend_from_slice(source),
            Conversion::Managed(transform) => {
                let offset = output.len();
                output.resize(
                    offset
                        + source.len() / self.source_components
                            * self.output.channels().unwrap_or(3) as usize,
                    0,
                );
                transform.apply(source, &mut output[offset..]);
            }
            Conversion::Proof(transform) => {
                output.extend(transform.convert(source).expect("complete print pixels"))
            }
        }
    }

    fn raster(self, width: u32, height: u32, samples: Vec<u8>) -> PrintRaster {
        PrintRaster {
            width,
            height,
            components: self.output.channels().unwrap_or(3),
            samples,
            profile: if self.tagged {
                self.output.to_icc()
            } else {
                Vec::new()
            },
            error: QString::default(),
        }
    }
}

fn working_rgb(doc: &Document) -> Profile {
    doc.source
        .working_profile()
        .as_deref()
        .and_then(|bytes| Profile::from_icc(bytes).ok())
        .filter(|profile| {
            profile.channels() == Some(3)
                && SpaceTransform::new(
                    profile,
                    &Profile::srgb(),
                    Intent::RelativeColorimetric,
                    false,
                )
                .is_ok()
        })
        .unwrap_or_else(Profile::srgb)
}

fn flat_native_source(doc: &Document) -> Option<(&SourcePlanes, Profile, bool)> {
    if !doc.layers.is_empty() || !doc.channels.is_empty() {
        return None;
    }
    let planes = doc.source.planes().as_ref()?;
    if (planes.width, planes.height) != (doc.width, doc.height) {
        return None;
    }
    let (components, default, inverted): (usize, fn() -> Profile, bool) =
        match doc.document_color_mode() {
            ColorMode::Cmyk => (4, Profile::coated_cmyk, true),
            ColorMode::Grayscale => (1, Profile::gray_gamma22, false),
            ColorMode::Rgb => (3, Profile::srgb, false),
            _ => return None,
        };
    if planes.samples.len() != doc.width as usize * doc.height as usize * components {
        return None;
    }
    let profile = doc
        .source
        .original_profile()
        .as_deref()
        .or(doc.source.working_profile().as_deref())
        .and_then(|bytes| Profile::from_icc(bytes).ok())
        .filter(|profile| {
            profile.channels() == Some(components as u8)
                && SpaceTransform::new(
                    profile,
                    &Profile::srgb(),
                    Intent::RelativeColorimetric,
                    false,
                )
                .is_ok()
        })
        .unwrap_or_else(default);
    Some((planes, profile, inverted))
}

pub(super) fn print_document_raster(
    view: &PictureView,
    output: &[u8],
    proof: &[u8],
    settings: &QString,
) -> PrintRaster {
    let view = view.rust();
    let Some(doc) = &view.session.doc else {
        return empty_raster("There is no document to print.");
    };
    let settings = parse(settings);
    let native = flat_native_source(doc);
    let working = working_rgb(doc);
    let source = native.as_ref().map_or(&working, |(_, profile, _)| profile);
    let conversion = match PrintConversion::new(source, output, proof, &settings) {
        Ok(conversion) => conversion,
        Err(error) => return empty_raster(error),
    };
    let mut samples = Vec::new();
    if let Some((planes, _, inverted)) = native {
        let plane = doc.width as usize * doc.height as usize;
        let mut first = 0;
        while first < plane {
            let count = (plane - first).min(doc.width as usize * 64);
            let mut band = vec![0; count * conversion.source_components];
            for channel in 0..conversion.source_components {
                let start = channel * plane + first;
                let Some(samples) = planes.samples.slice(start..start + count) else {
                    return empty_raster("The native print samples could not be read.");
                };
                for (pixel, value) in band
                    .chunks_exact_mut(conversion.source_components)
                    .zip(samples.narrow_to_u8())
                {
                    pixel[channel] = if inverted { 255 - value } else { value };
                }
            }
            conversion.append_pixels(&band, &mut samples);
            first += count;
        }
    } else {
        picture::stream_picture(doc, view, picture::whole(doc), |_, band| {
            conversion.append(
                &helpers_composite::buffer_to_rgba_bytes(&band),
                &mut samples,
            );
        });
    }
    conversion.raster(doc.width, doc.height, samples)
}

pub(super) fn print_image_raster(
    rgba: &[u8],
    width: u32,
    height: u32,
    output: &[u8],
    proof: &[u8],
    settings: &QString,
) -> PrintRaster {
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        != Some(rgba.len())
    {
        return empty_raster("Invalid print image dimensions.");
    }
    let conversion = match PrintConversion::new(&Profile::srgb(), output, proof, &parse(settings)) {
        Ok(conversion) => conversion,
        Err(error) => return empty_raster(error),
    };
    let mut samples = Vec::new();
    conversion.append(rgba, &mut samples);
    conversion.raster(width, height, samples)
}

fn print_raster_display(raster: &PrintRaster) -> PrintRaster {
    let profile = if raster.profile.is_empty() {
        Profile::srgb()
    } else {
        match Profile::from_icc(&raster.profile) {
            Ok(profile) => profile,
            Err(error) => return empty_raster(error.to_string()),
        }
    };
    let rgb = match convert_space(
        &profile,
        &Profile::srgb(),
        &raster.samples,
        Intent::RelativeColorimetric,
        false,
    ) {
        Ok(rgb) => rgb,
        Err(error) => return empty_raster(error.to_string()),
    };
    let rgba = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[r, g, b]| [r, g, b, 255])
        .collect();
    rgb_raster(raster.width, raster.height, rgba)
}

pub(super) fn print_document_preview(
    view: &PictureView,
    max_side: u32,
    output: &[u8],
    proof: &[u8],
    settings: &QString,
) -> PrintRaster {
    let view = view.rust();
    let Some(doc) = &view.session.doc else {
        return empty_raster("There is no document to print.");
    };
    let reduced = picture::preview_picture(doc, view, max_side.max(1));
    let width = reduced.width;
    let height = reduced.height;
    let native = flat_native_source(doc);
    let working = working_rgb(doc);
    let source = native.as_ref().map_or(&working, |(_, profile, _)| profile);
    let conversion = match PrintConversion::new(source, output, proof, &parse(settings)) {
        Ok(conversion) => conversion,
        Err(error) => return empty_raster(error),
    };
    let mut samples = Vec::new();
    let pixels = if let Some((planes, _, inverted)) = &native {
        let mut pixels = vec![0; width as usize * height as usize * conversion.source_components];
        for y in 0..height {
            let sy = u64::from(y) * u64::from(doc.height) / u64::from(height);
            for channel in 0..conversion.source_components {
                let Some(row) = planes.samples.row(
                    channel * doc.height as usize + sy as usize,
                    doc.width as usize,
                ) else {
                    return empty_raster("The native print samples could not be read.");
                };
                for x in 0..width {
                    let sx = (u64::from(x) * u64::from(doc.width) / u64::from(width)) as usize;
                    let value = match row {
                        SampleRow::U8(values) => values.get(sx).copied(),
                        SampleRow::U16(values) => values.get(sx).map(|value| (value >> 8) as u8),
                        SampleRow::F32(values) => values
                            .get(sx)
                            .map(|value| (value * 256.0).trunc().clamp(0.0, 255.0) as u8),
                    };
                    let Some(value) = value else {
                        return empty_raster("The native print samples could not be read.");
                    };
                    let index = (y as usize * width as usize + x as usize)
                        * conversion.source_components
                        + channel;
                    pixels[index] = if *inverted { 255 - value } else { value };
                }
            }
        }
        pixels
    } else {
        conversion.flatten(&helpers_composite::buffer_to_rgba_bytes(&reduced))
    };
    conversion.append_pixels(&pixels, &mut samples);
    let settings = parse(settings);
    let mask = if settings["managed"].as_bool() == Some(true)
        && settings["gamut"].as_bool() == Some(true)
    {
        let setup = ProofSetup {
            device: Profile::from_icc(output).expect("validated output profile"),
            intent: output_options(&settings).0,
            bpc: output_options(&settings).1,
            simulate_paper: false,
            simulate_black: false,
        };
        print_gamut_mask_source(&pixels, source, &setup).ok()
    } else {
        None
    };
    let mut result = print_raster_display(&conversion.raster(width, height, samples));
    if let Some(mask) = mask {
        let (pixels, _) = result.samples.as_chunks_mut::<4>();
        for (pixel, warning) in pixels.iter_mut().zip(mask) {
            if warning {
                pixel[..3].copy_from_slice(&[230, 0, 230]);
            }
        }
    }
    result
}

pub(super) fn print_pdf_bytes(
    raster: &PrintRaster,
    geometry: &[f64],
    background: &QString,
    marks: &QString,
) -> Vec<u8> {
    let &[page_w, page_h, x, y, width, height] = geometry else {
        return Vec::new();
    };
    write_print_pdf(&PrintPage {
        size: [page_w, page_h],
        image_rect: [x, y, width, height],
        image: PrintImage {
            width: raster.width,
            height: raster.height,
            components: raster.components,
            samples: &raster.samples,
            icc: (!raster.profile.is_empty()).then_some(raster.profile.as_slice()),
        },
        background: &background.to_string(),
        marks: &marks.to_string(),
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_gamut_preview_uses_the_print_conversion_and_matte() {
        let source = Profile::adobe_rgb();
        let output = Profile::coated_cmyk();
        let options = json!({"managed": true, "intent": 1, "bpc": true,
                             "background": [30, 70, 180]});
        let rgba = [20, 240, 40, 160];
        let conversion = PrintConversion::new(&source, &output.to_icc(), &[], &options).unwrap();
        let mut samples = Vec::new();
        conversion.append(&rgba, &mut samples);
        let raster = conversion.raster(1, 1, samples.clone());
        let display = print_raster_display(&raster);
        let expected = convert_space(
            &output,
            &Profile::srgb(),
            &samples,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        assert_eq!(&display.samples[..3], expected.as_slice());
        let clipped = convert_space(
            &source,
            &Profile::srgb(),
            &rgba[..3],
            Intent::RelativeColorimetric,
            true,
        )
        .unwrap();
        let wrong = convert_space(
            &Profile::srgb(),
            &output,
            &clipped,
            Intent::RelativeColorimetric,
            true,
        )
        .unwrap();
        assert_ne!(samples, wrong);
    }
}
