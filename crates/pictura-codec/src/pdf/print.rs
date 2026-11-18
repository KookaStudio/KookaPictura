use super::{
    color_space, flate, object, write_plain, Body, Built, Dict, Obj, Objects, Space, PRODUCER,
};
use crate::PsdError;

#[derive(Clone, Copy)]
pub struct PrintImage<'a> {
    pub width: u32,
    pub height: u32,
    pub components: u8,
    pub samples: &'a [u8],
    pub icc: Option<&'a [u8]>,
}

pub struct PrintPage<'a> {
    pub size: [f64; 2],
    /// Left, bottom, width, height in PDF points.
    pub image_rect: [f64; 4],
    pub image: PrintImage<'a>,
    /// Page drawing operators produced by the print layout, in page coordinates.
    pub background: &'a str,
    pub marks: &'a str,
}

/// Encode a single print page with lossless device samples.
pub fn write_print_pdf(page: &PrintPage<'_>) -> Result<Vec<u8>, PsdError> {
    let PrintImage {
        width,
        height,
        components,
        samples,
        icc,
    } = page.image;
    let [page_w, page_h] = page.size;
    let [x, y, w, h] = page.image_rect;
    if width == 0
        || height == 0
        || (width as usize)
            .checked_mul(height as usize)
            .and_then(|count| count.checked_mul(components as usize))
            != Some(samples.len())
        || page
            .size
            .iter()
            .any(|side| !side.is_finite() || *side <= 0.0)
        || page.image_rect.iter().any(|v| !v.is_finite())
        || x < 0.0
        || y < 0.0
        || w <= 0.0
        || h <= 0.0
        || x + w > page_w + 0.001
        || y + h > page_h + 0.001
    {
        return Err(PsdError::Invalid("invalid print page or image size".into()));
    }
    let device = match components {
        1 => Space::Gray,
        3 => Space::Rgb,
        4 => Space::Cmyk,
        _ => return Err(PsdError::Invalid("invalid printer colour space".into())),
    };
    let space = icc.map_or(device, |bytes| Space::Icc(components, bytes.to_vec()));
    // PDF 1.7 can carry the original version-4 ICC bytes.
    let minor = 7;
    let mut objects = Objects::default();
    let catalog = objects.reserve();
    let pages = objects.reserve();
    let page_id = objects.reserve();
    let mut page_objects = vec![page_id];
    let colorspace = color_space(&mut objects, &mut page_objects, &space, minor);
    let image = objects.add(Body::Stream(
        Dict::typed("XObject")
            .with("Subtype", Obj::name("Image"))
            .with("Width", Obj::Int(i64::from(width)))
            .with("Height", Obj::Int(i64::from(height)))
            .with("BitsPerComponent", Obj::Int(8))
            .with("Filter", Obj::name("FlateDecode"))
            .with("ColorSpace", colorspace.clone()),
        flate(samples),
    ));
    let operators = format!(
        "q\n{}\nQ\nq {} 0 0 {} {} {} cm /Im0 Do Q\nq\n{}\nQ\n",
        page.background,
        object::real(w),
        object::real(h),
        object::real(x),
        object::real(y),
        page.marks,
    );
    let content = objects.add(Body::Stream(Dict::new(), operators.into_bytes()));
    page_objects.extend([image, content]);
    let resources = Dict::new()
        .with("XObject", Dict::new().with("Im0", Obj::Ref(image)).into())
        .with("ColorSpace", Dict::new().with("Print", colorspace).into());
    objects.set(
        page_id,
        Body::Obj(
            Dict::typed("Page")
                .with("Parent", Obj::Ref(pages))
                .with("MediaBox", Obj::reals(&[0.0, 0.0, page_w, page_h]))
                .with("Resources", resources.into())
                .with("Contents", Obj::Ref(content))
                .into(),
        ),
    );
    objects.set(
        pages,
        Body::Obj(
            Dict::typed("Pages")
                .with("Kids", Obj::Array(vec![Obj::Ref(page_id)]))
                .with("Count", Obj::Int(1))
                .into(),
        ),
    );
    objects.set(
        catalog,
        Body::Obj(Dict::typed("Catalog").with("Pages", Obj::Ref(pages)).into()),
    );
    let info = objects.add(Body::Obj(
        Dict::new().with("Producer", Obj::text(PRODUCER)).into(),
    ));
    Ok(write_plain(&Built {
        objects,
        catalog,
        info,
        page: page_id,
        content,
        page_objects,
        encrypt: None,
        encryptor: None,
        id: [0; 16],
        minor,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::tests::{image, inflate, stream, text};
    use pictura_color::Profile;

    #[test]
    fn print_pdf_preserves_icc_device_samples() {
        for profile in [
            Profile::gray_gamma22(),
            Profile::adobe_rgb(),
            Profile::coated_cmyk(),
        ] {
            let icc = profile.to_icc();
            let components = profile.channels().unwrap();
            let samples = &b"\x11\x32\x54\x76\x98\xac\xdf\x0e"[..2 * components as usize];
            let pdf = write_print_pdf(&PrintPage {
                size: [595.28, 841.89],
                image_rect: [36.0, 36.0, 360.0, 180.0],
                image: PrintImage {
                    width: 2,
                    height: 1,
                    components,
                    samples,
                    icc: Some(&icc),
                },
                background: "",
                marks: "",
            })
            .unwrap();
            let (dict, data) = image(&pdf);
            assert!(dict.contains("/ICCBased"));
            assert_eq!(data, samples);
            let (_, stream) = stream(&pdf, &[&format!("/N {components}"), "/Alternate"]).unwrap();
            assert_eq!(inflate(stream), icc);
            assert!(text(&pdf).contains("/MediaBox [0 0 595.28 841.89]"));
        }
    }
}
