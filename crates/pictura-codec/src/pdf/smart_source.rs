//! Prepare an embedded PDF page using the bounded PDF renderer.

use std::path::{Path, PathBuf};

use pictura_core::VectorPlacement;

use super::import::{self, CropTo, PageInfo};
use crate::ghostscript::ScratchDir;
use crate::{sniff_vector, Ghostscript, Raster, RasterError, VectorKind};

pub struct SmartPdf {
    _scratch: ScratchDir,
    path: PathBuf,
    gs: Ghostscript,
    pages: Vec<PageInfo>,
}

impl SmartPdf {
    pub fn open(bytes: &[u8]) -> Result<Self, RasterError> {
        if sniff_vector(bytes) != Some(VectorKind::Pdf) {
            return Err(RasterError::Failed(
                "the source is not PDF-compatible".into(),
            ));
        }
        let gs = Ghostscript::locate()?;
        let scratch = ScratchDir::new()?;
        let path = scratch.path("source.pdf");
        std::fs::write(&path, bytes).map_err(|error| RasterError::Failed(error.to_string()))?;
        let pages = import::page_info(&gs, &path, None)?;
        Ok(Self {
            _scratch: scratch,
            path,
            gs,
            pages,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn prepare(&mut self, page: u32, crop: u8) -> Result<VectorPlacement, RasterError> {
        if crop > 5 {
            return Err(RasterError::Failed("unknown page box".into()));
        }
        if crop == 0 {
            import::query_bounding_boxes(&self.gs, &self.path, None, &mut self.pages)?;
        }
        let area = self
            .page(page)?
            .view_box(CropTo::from_index(i32::from(crop)));
        let (width_points, height_points) = (area.width(), area.height());
        if !width_points.is_finite()
            || !height_points.is_finite()
            || width_points <= 0.0
            || height_points <= 0.0
        {
            return Err(RasterError::Failed("the selected page box is empty".into()));
        }
        Ok(VectorPlacement {
            page,
            page_count: self.pages.len() as u32,
            crop,
            width_points,
            height_points,
        })
    }

    pub fn render(
        &self,
        placement: VectorPlacement,
        size: (u32, u32),
    ) -> Result<Raster, RasterError> {
        Ok(import::render_page(
            &self.gs,
            &self.path,
            None,
            placement.page,
            self.page(placement.page)?,
            CropTo::from_index(i32::from(placement.crop)),
            size,
            true,
        )?
        .raster)
    }

    fn page(&self, page: u32) -> Result<&PageInfo, RasterError> {
        page.checked_sub(1)
            .and_then(|index| self.pages.get(index as usize))
            .ok_or_else(|| RasterError::Failed("the selected page is missing".into()))
    }
}

pub fn filetype(filename: &str) -> [u8; 4] {
    if Path::new(filename)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ai"))
    {
        *b"AI  "
    } else {
        *b"PDF "
    }
}
