use pictura_codec::pdf::smart_source::SmartPdf;
use pictura_codec::Raster;
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PsdRect, VectorPlacement,
    VectorTransform,
};
use pictura_render::Interpolation;

pub(super) fn corners(rect: PsdRect) -> [(f64, f64); 4] {
    let (left, top, right, bottom) = (
        f64::from(rect.left),
        f64::from(rect.top),
        f64::from(rect.right),
        f64::from(rect.bottom),
    );
    [(left, top), (right, top), (right, bottom), (left, bottom)]
}

pub(super) fn channels(raster: &Raster) -> Vec<Channel> {
    [0, 1, 2, -1]
        .into_iter()
        .enumerate()
        .map(|(channel, id)| Channel {
            id,
            data: raster.rgba[channel..]
                .iter()
                .step_by(4)
                .copied()
                .collect::<Vec<_>>()
                .into(),
        })
        .collect()
}

pub(super) fn install_proxy(layer: &mut Layer, channels: Vec<Channel>) -> Option<()> {
    layer.set_channels(pictura_render::bake_smart_source_preview(layer, channels)?);
    layer.discard_raw_channels();
    layer.source_channels = None;
    Some(())
}

pub(super) fn project(
    raster: Raster,
    rect: PsdRect,
    transforms: &[VectorTransform],
    clip_placement: bool,
) -> Option<Vec<Channel>> {
    if transforms.is_empty() && size(rect)? == (raster.width, raster.height) {
        return Some(channels(&raster));
    }
    let initial = transforms.first().map_or(rect, |step| step.source);
    let mut scratch = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    let path = pictura_render::add_raster_layer_from_rgba(
        &mut scratch,
        "",
        raster.width,
        raster.height,
        &raster.rgba,
    );
    reshape(&mut scratch, &path, initial)?;
    for (index, step) in transforms.iter().enumerate() {
        reshape(&mut scratch, &path, step.source)?;
        let changed = match &step.warp {
            Some(warp) => pictura_render::transform_layer_warp(
                &mut scratch,
                &path,
                warp,
                Some(step.quad),
                Interpolation::Bilinear,
            ),
            None if corners(step.source) == step.quad => true,
            None => pictura_render::transform_layer_quad_with(
                &mut scratch,
                &path,
                step.quad,
                Interpolation::Bilinear,
            ),
        };
        if !changed {
            return None;
        }
        if clip_placement && index == 0 {
            let layer = pictura_render::resolve_path_mut(&mut scratch, &path)?;
            let target = transforms.get(1).map_or(rect, |step| step.source);
            if size(layer.rect)? != size(target)? {
                layer.set_channels(clip(layer.channels(), layer.rect, target)?);
            }
            layer.rect = target;
        }
    }
    reshape(&mut scratch, &path, rect)?;
    Some(std::mem::take(
        pictura_render::resolve_path_mut(&mut scratch, &path)?.channels_mut(),
    ))
}

fn clip(channels: &[Channel], source: PsdRect, target: PsdRect) -> Option<Vec<Channel>> {
    let (sw, _) = size(source)?;
    let (tw, th) = size(target)?;
    let len = (tw as usize).checked_mul(th as usize)?;
    let left = source.left.max(target.left);
    let right = source.right.min(target.right);
    let top = source.top.max(target.top);
    let bottom = source.bottom.min(target.bottom);
    channels
        .iter()
        .map(|channel| {
            let mut data = Vec::new();
            data.try_reserve_exact(len).ok()?;
            data.resize(len, 0);
            if right > left && bottom > top {
                let width = (right - left) as usize;
                for y in top..bottom {
                    let from =
                        (y - source.top) as usize * sw as usize + (left - source.left) as usize;
                    let to =
                        (y - target.top) as usize * tw as usize + (left - target.left) as usize;
                    channel.data.read_into(from, &mut data[to..to + width]);
                }
            }
            Some(Channel {
                id: channel.id,
                data: data.into(),
            })
        })
        .collect()
}

fn reshape(doc: &mut Document, path: &str, rect: PsdRect) -> Option<()> {
    let layer = pictura_render::resolve_path_mut(doc, path)?;
    if size(layer.rect)? == size(rect)? {
        layer.rect = rect;
        return Some(());
    }
    pictura_render::transform_layer_quad_with(doc, path, corners(rect), Interpolation::Bilinear)
        .then_some(())
}

pub(super) fn size(rect: PsdRect) -> Option<(u32, u32)> {
    let width = u32::try_from(rect.right.checked_sub(rect.left)?).ok()?;
    let height = u32::try_from(rect.bottom.checked_sub(rect.top)?).ok()?;
    (width != 0 && height != 0).then_some((width, height))
}

pub(super) fn render(
    pdf: &SmartPdf,
    page: VectorPlacement,
    rect: PsdRect,
    transforms: &[VectorTransform],
) -> Option<Vec<Channel>> {
    render_projected(pdf, page, rect, transforms, false)
}

pub(super) fn render_projected(
    pdf: &SmartPdf,
    page: VectorPlacement,
    rect: PsdRect,
    transforms: &[VectorTransform],
    clip_placement: bool,
) -> Option<Vec<Channel>> {
    let mut density = [1.0_f64; 2];
    let mut steps = Vec::with_capacity(transforms.len());
    let mut output = rect;
    for step in transforms.iter().rev() {
        let (width, height) = size(step.source)?;
        let [top_left, top_right, bottom_right, bottom_left] = step.quad;
        let edge = |a: (f64, f64), b: (f64, f64)| {
            ((a.0 - b.0) * density[0]).hypot((a.1 - b.1) * density[1])
        };
        let input_density = [
            (edge(top_left, top_right).max(edge(bottom_left, bottom_right)) / f64::from(width))
                .max(1.0),
            (edge(top_left, bottom_left).max(edge(top_right, bottom_right)) / f64::from(height))
                .max(1.0),
        ];
        // Scale upstream projection frames as well as the PDF raster. The
        // last stage still maps to destination pixels at their normal scale.
        let grid = |density: f64| (density - 1e-9).ceil().max(1.0);
        let input_grid = input_density.map(grid);
        let output_grid = density.map(grid);
        steps.push(project_at_density(step, input_grid, output_grid, output)?);
        density = input_density;
        output = step.source;
    }
    steps.reverse();
    let size = size(steps.first().map_or(rect, |step| step.source))?;
    let raster = pdf.render(page, size).ok()?;
    project(raster, rect, &steps, clip_placement)
}

fn project_at_density(
    step: &VectorTransform,
    input_grid: [f64; 2],
    output_grid: [f64; 2],
    output: PsdRect,
) -> Option<VectorTransform> {
    let source = step.source;
    let mut warp = step.warp.clone();
    let source = if input_grid == [1.0, 1.0] {
        source
    } else {
        let (width, height) = size(source)?;
        let (left, top) = (f64::from(source.left), f64::from(source.top));
        let right = f64::from(width) * input_grid[0];
        let bottom = f64::from(height) * input_grid[1];
        if right > f64::from(i32::MAX) || bottom > f64::from(i32::MAX) {
            return None;
        }
        if let Some(warp) = &mut warp {
            let scale = |x, y| [(x - left) * input_grid[0], (y - top) * input_grid[1]];
            let [x0, y0, x1, y1] = warp.bounds;
            let [x0, y0] = scale(x0, y0);
            let [x1, y1] = scale(x1, y1);
            warp.bounds = [x0, y0, x1, y1];
            if let Some(mesh) = &mut warp.mesh {
                for point in &mut mesh.points {
                    *point = scale(point[0], point[1]);
                }
            }
        }
        PsdRect {
            left: 0,
            top: 0,
            right: right as i32,
            bottom: bottom as i32,
        }
    };
    let quad = if output_grid == [1.0, 1.0] {
        step.quad
    } else {
        let (left, top) = (f64::from(output.left), f64::from(output.top));
        step.quad
            .map(|(x, y)| ((x - left) * output_grid[0], (y - top) * output_grid[1]))
    };
    Some(VectorTransform {
        source,
        destination: None,
        quad,
        warp,
    })
}

pub(super) fn source_instances<'a>(stack: &'a [Layer], id: &str) -> Vec<&'a Layer> {
    fn collect<'a>(layers: &'a [Layer], id: &str, result: &mut Vec<&'a Layer>) {
        for layer in layers {
            if layer.smart_object().is_some_and(|object| object.uuid == id) {
                result.push(layer);
            }
            collect(layer.children(), id, result);
        }
    }
    let mut result = Vec::new();
    collect(stack, id, &mut result);
    result
}

pub(super) fn update_source_instances(
    stack: &mut LayerStack,
    id: &str,
    update: &mut impl FnMut(&mut Layer) -> Option<()>,
) -> Option<()> {
    for layer in stack.iter_mut() {
        if layer.smart_object().is_some_and(|object| object.uuid == id) {
            update(layer)?;
        }
        if let Some(children) = layer.children_mut() {
            update_source_instances(children, id, update)?;
        }
    }
    Some(())
}
