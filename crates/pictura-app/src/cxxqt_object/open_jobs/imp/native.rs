use super::*;

/// Read and decode a PSD/PSB. `None` when the file is missing or unsupported.
pub(in crate::cxxqt_object) fn decode_psd(
    path: &str,
    policy: Policy,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    decode_psd_with(path, open_options(policy), open_budget(), gpu, progress).ok()
}

/// Whether the last PSD open was refused because its decode did not fit the
/// memory governor's room. Set on the open's worker, read on the GUI thread.
pub(super) static OPEN_REFUSED_FOR_MEMORY: AtomicBool = AtomicBool::new(false);

pub(in crate::cxxqt_object) fn open_refused_for_memory() -> bool {
    OPEN_REFUSED_FOR_MEMORY.load(Ordering::Relaxed)
}

/// The decode budget of an open, read on the calling (GUI) thread, and
/// whether the governor's room bounded it. Contiguous allocations get two
/// thirds of the room (the rest is the composite, history and display the open
/// builds after the decode, #1211). The layer tiles get that plus the scratch
/// disks' free space, since the evictor can pack and spill them (#1273).
pub(in crate::cxxqt_object) fn open_budget() -> (pictura_codec::ImageBudget, bool) {
    let default = pictura_codec::ImageBudget::default();
    let Some(room) = pictura_core::memory::governor()
        .room()
        .map(|room| room / 3 * 2)
    else {
        return (default, false);
    };
    let mut budget = super::super::super::helpers::bounded_budget(default, Some(room));
    budget.max_tiled_bytes =
        room.saturating_add(super::super::super::helpers::memory_gate::scratch_available());
    (budget, true)
}

fn decode_psd_with(
    path: &str,
    options: pictura_codec::OpenOptions,
    budget: (pictura_codec::ImageBudget, bool),
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Result<OpenedDocument, Error> {
    let mut memory_refused = false;
    let result = decode_native(path, options, budget, gpu, progress, &mut memory_refused);
    OPEN_REFUSED_FOR_MEMORY.store(memory_refused, Ordering::Relaxed);
    result
}

pub(super) fn decode_native(
    path: &str,
    mut options: pictura_codec::OpenOptions,
    (budget, bounded): (pictura_codec::ImageBudget, bool),
    gpu: bool,
    progress: &dyn Fn(u8, &str),
    memory_refused: &mut bool,
) -> Result<OpenedDocument, Error> {
    progress(5, "Reading file");
    // The file stays open behind the document: its 8-bit raw and RLE layer
    // channels are read from it on demand (#1218). Without a pin (no
    // positioned reads on this platform) the open decodes everything.
    let pin = pictura_codec::SourcePin::open(path).ok();
    let bytes = match pin.as_ref().map(|pin| pin.map()) {
        Some(mapped) => mapped,
        None => pictura_codec::MappedFile::open(path),
    }
    .map_err(|error| Error::with_source(ErrorCode::Io, error.to_string(), error))?;
    options.source = pin.clone();
    progress(25, "Decoding");
    let (mut doc, preview) = match pictura_codec::read_psd_lazy(&bytes, &options, budget) {
        Ok(read) => read,
        Err(error) => {
            // The codec names a budget refusal "… exceeds the decode budget".
            let memory = bounded && error.to_string().contains("decode budget");
            *memory_refused = memory;
            if memory
                || matches!(
                    error,
                    pictura_codec::PsdError::NativeVersion(_)
                        | pictura_codec::PsdError::BadSignature(_)
                )
            {
                return Err(Error::with_source(
                    ErrorCode::Codec,
                    error.to_string(),
                    error,
                ));
            }
            let Some(doc) = pictura_codec::read_psd_layer_structure(&bytes, &options, budget)
                .ok()
                .filter(|doc| {
                    native_access(doc, &pictura_codec::document_version(doc)).is_read_only()
                })
            else {
                return Err(Error::with_source(
                    ErrorCode::Codec,
                    error.to_string(),
                    error,
                ));
            };
            (doc, None)
        }
    };
    let version = pictura_codec::document_version(&doc);
    let access = native_access(&doc, &version);
    let compatibility = if access.is_read_only() {
        let has_real_merge = version
            .version_info
            .as_ref()
            .is_some_and(|info| info.has_real_merged_data)
            || version.version_info.is_none() && doc.layers.is_empty();
        let merged_alpha = pictura_codec::native_merged_has_transparency(&bytes, doc.is_psb);
        let pixels = if !has_real_merge || !doc.composite.merged_output() {
            None
        } else if let Some(pixels) = source_pixels(&doc, merged_alpha) {
            Some(pixels)
        } else {
            options.skip_layered_merged = false;
            pictura_codec::read_psd_within_options(&bytes, &options, budget)
                .ok()
                .and_then(|source| source_pixels(&source, merged_alpha))
        };
        NativeCompatibility::new(access, Path::new(path), pin, pixels)
    } else {
        NativeCompatibility::new(access, Path::new(path), None, None)
    };
    progress(65, "Compositing");
    if !compatibility.access.is_read_only() {
        store_opened_composite(&mut doc, gpu);
    }
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, None);
    let history = history_for(&mut doc, &target, "Open");
    Ok(OpenedDocument {
        preview,
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "psd".to_string(),
        pdf_password: None,
        compatibility,
    })
}
