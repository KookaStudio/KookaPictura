use super::*;

impl Stroke {
    pub(super) fn apply_per_dab(
        &mut self,
        doc: &mut Document,
        dabs: &[(f32, f32)],
        tangent: f32,
    ) -> bool {
        let cfg = self.cfg;
        // The composite engines keep their own alpha, but a layer without an
        // alpha plane cannot store Clear: match the regular raster's no-op.
        if cfg.mode == PaintMode::Clear && self.ch[3].is_none() {
            return false;
        }
        let Some(per) = self.per_dab.as_mut() else {
            return false;
        };
        let mut changed = Vec::with_capacity(dabs.len());
        for &(x, y) in dabs {
            if let Some(lazy) = per.lazy.as_mut() {
                let Some(layer) = layer_at(doc, &self.layer_path) else {
                    return false;
                };
                if !self.saved.fits(layer, self.ch) {
                    return false;
                }
                if let Some(region) = dab_bounds(&per.pixels, &cfg, x, y) {
                    lazy.load(layer, &mut per.pixels, region);
                }
            }
            let dirty = match &mut per.engine {
                DabEngine::Replace(replacer) => {
                    replacer.dab(&mut per.pixels, &cfg, x, y, cfg.color)
                }
                DabEngine::Mixer(mixer) => mixer.dab_tangent(&mut per.pixels, &cfg, x, y, tangent),
                DabEngine::ArtHistory(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::BackgroundErase(eraser) => {
                    eraser.dab(&mut per.pixels, &cfg, x, y, cfg.color)
                }
                DabEngine::Focus(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::Smudge(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::Tone(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
            };
            changed.extend(dirty);
        }
        if changed.is_empty() {
            return false;
        }
        let clip = self.coverage_clip.as_ref();
        stencil::mask_dabs(
            clip,
            self.rect,
            doc,
            &self.layer_path,
            &mut per.pixels,
            &changed,
        );
        let ch = self.ch;
        let Some(layer) = layer_at_mut(doc, &self.layer_path).filter(|l| self.saved.fits(l, ch))
        else {
            return false;
        };
        // Each dab's own box: on a fast drag the union of a sample's dabs is
        // mostly pixels no dab reached.
        let mut written: Option<PsdRect> = None;
        for rect in changed {
            self.saved.save(
                layer,
                ch,
                rect.left,
                rect.top,
                rect.right - 1,
                rect.bottom - 1,
            );
            let bbox = write_back(
                layer,
                ch,
                &per.pixels,
                rect,
                &mut self.scratch,
                &mut self.applied,
                self.serial,
            );
            if let Some(bbox) = bbox {
                written = Some(written.map_or(bbox, |w| w.union(bbox)));
            }
        }
        let Some(rect) = written else {
            return false;
        };
        Self::grow_dirty(&mut self.dirty, &mut self.dab_dirty, rect.left, rect.top);
        Self::grow_dirty(
            &mut self.dirty,
            &mut self.dab_dirty,
            rect.right - 1,
            rect.bottom - 1,
        );
        self.painted = true;
        true
    }
}
