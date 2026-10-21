# Timeline Panel

- **Spec ID:** `TOOL-062`
- **Status:** `Draft`
- **Parity tier:** `Non-goal (Linux)` for exact CS6 Timeline parity; the CS6 source feature is `Core` (ships in **all** editions). A limited FFmpeg-backed decode/import path is the revisit condition (`OVR-003`).
- **New in CS6:** `Changed` — the CS5 **Animation panel** was redesigned as a **clip-based Timeline panel** and the video timeline became available in **all** CS6 editions (it was Extended-only in CS5); Video Groups, audio tracks, transitions, and clip speed/duration controls were added. Frame-animation mode was retained.
- **Depends on:** `OVR-002` cs6-editions-and-constraints, `OVR-003` feasibility-and-non-goals, `ARCH-002` document-model, `ARCH-003` qt6-ui-design, `ARCH-006` gpu-rendering-pipeline, `ARCH-008` undo-history, `TOOL-010` layers-panel, `10-workflow-io/export-formats.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Where the CS6 Help text and the CS6 release
> positioning disagree, both are reported and the conflict is carried into Open
> questions.

## CS6 behavior

In CS6 the panel is named the **Timeline panel** (`Window > Timeline`) and ships in **Standard and Extended**; the CS6 Help PDF still uses the CS5 name **Animation panel** for parts of its text and tags some timeline features "Photoshop Extended" — a CS5-era artifact (see Open questions). It has two modes.

### Frame animation mode ("frame timeline")

A thumbnail per frame; each frame is a **configuration of layers**, not a separate image. Controls: `Looping Options` (Once / 3 Times / Forever / Other, exported to animated GIF), `Frame Delay Time` (seconds, decimals), `Tween Animation Frames`, `Duplicate Selected Frames`, and `Convert To Timeline Animation`. Frames can be reordered, reversed, copied/pasted (Replace Frames, Paste Over Selection, Paste Before/After Selection, Link Added Layers), and given a **disposal method** (`Automatic`, `Do Not Dispose`, `Dispose`) for GIF transparency. `Unify Layer Position/Visibility/Style` and `Propagate Frame 1` push an attribute change across frames of a layer. `Make Frames From Layers` builds a frame animation from multi-layer PSDs; `Create New Layer For Each New Frame` speeds up per-frame content.

### Video timeline mode ("video timeline")

A Premiere-like, **clip-based** timeline. Every document layer (except the background) becomes a **track**, synchronised with the Layers panel. Features: **keyframes** on layer properties via the **Time-Vary stopwatch**, a **current-time indicator**, **time ruler** (frame numbers or timecode), **layer duration bars** that can be moved and trimmed, **work-area indicators**, a **keyframe navigator**, a **Global Lighting** track (master lighting angle for layer effects), an **Altered Video** track for edited video frames, a **Comments** track (with HTML export), **cached-frame** indicator, and **onion skinning** for hand-drawn work. Video layers only function in video timeline mode, not frame mode.

**Keyframes and tweening.** At least two keyframes are needed to animate a property; with the stopwatch active, moving the current time and changing a value inserts a keyframe automatically. Interpolation is `Linear` (even change) or `Hold` (constant step; Layer Mask Position snaps). Interpolation between keyframes is the timeline counterpart of frame mode's **Tween**.

**Clip operations.** Video Groups combine clips, text, images, and shapes on one track. Clips can be split at the playhead, reordered, trimmed (in/out points), speed-changed (`Duration`/`Speed` interact), and given `pan/zoom/rotate` presets that create Transform keyframes. **Transitions** (fade/cross-fade) are dragged onto clip edges and have numeric durations. Audio lives on separate tracks with mute, volume, fade-in/out, add/duplicate/delete/replace, and per-video-clip audio controls.

**Video layers.** Importing a video or image sequence creates a **video layer** (filmstrip icon) that references the original file; frames can be painted, cloned, masked, transformed, given layer styles/blending/adjustment layers, and individual frames rasterised. A **blank video layer** supports hand-drawn animation and rotoscoping. To apply a filter across a video layer it must first be converted to a **Smart Object**, after which the filter becomes a **Smart Filter**. `Layer > Video Layers` inserts/deletes/duplicates blank video frames. Supported video-layer colour modes/bpc: Grayscale 8/16/32, RGB 8/16/32, CMYK 8/16, Lab 8/16.

**Document settings and formats.** `Document Settings` sets `Duration` (default 10 s) and `Frame Rate` (30 fps for non-video presets; PAL 25, NTSC 29.97, film 24/23.976; web 10–15). Import formats include 3GP/3G2, AVI, DV, FLV/F4V, MPEG-1/4, MOV, and WAV (full MOV support on Windows required a separate QuickTime install). Export goes through **Adobe Media Encoder** (`File > Export > Render Video`): DPX (frame sequences), H.264 (MPEG-4, with HD/web presets), and QuickTime MOV (required for alpha and uncompressed output). Frame animations can also be saved as animated GIF via Save For Web, an image sequence, or a QuickTime movie.

### Extended-only and non-goal status

CS6's clip-based Timeline is a **Core** feature of every edition. Kooka Pictura nevertheless treats **exact CS6 timeline parity** as a `Non-goal (Linux)` (`OVR-003`): the timeline semantics, QuickTime/DV/FLV legacy codecs, and Media Encoder are Adobe/Apple-specific. The revisit condition is a **decode/import** feature built on FFmpeg, without claiming CS6 timeline parity. A `Core` build must still **represent and preserve** video/animation layers so CS6 files open and re-save without loss.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Timeline` | Menu → panel | — | CS6 panel name (CS5 PDF: "Animation") |
| Panel bottom-left | Playback controls | `Spacebar` | Play/stop; first/prev/next/last frame |
| Panel bottom-left | Looping option box | — | Frame mode: Once / 3 Times / Forever / Other |
| Panel bottom | Onion-skin toggle | — | Timeline + frame drawing |
| Panel bottom | Delete keyframes / zoom / zoom slider | — | Timeline mode |
| Panel menu | Fly-out menu | — | Keyframes, layers, onion skin, document settings |
| Panel menu → `Document Settings` | Dialog | — | Duration + frame rate |
| Panel menu → `Panel Options` | Dialog | — | Thumbnails, Frame Numbers vs Timecode |
| Panel menu → `Set Favorite Layers` / `Show` | Menu | — | Show All vs Favorite layers |
| `Animation/Video Timeline` convert icon | Button | — | Convert frame ↔ video timeline |
| Timeline top | Transitions icon | — | Drag fade/cross-fade onto clip edges |
| Timeline track names | Musical-note menu | — | New/Delete track; Add/Duplicate/Delete/Replace audio |
| Timeline clip | Right-click | — | Mute/volume/fade; Duration & Speed; transition options |
| Keyframes | Right-click | — | Linear / Hold interpolation; copy/delete keyframes |
| `Layer > Video Layers` | Menu | — | Insert/Delete/Duplicate blank video frame |
| `Layer > Smart Objects > Convert to Smart Object` | Menu | — | Required before filtering a video layer |
| `File > Export > Render Video` | Menu | — | Adobe Media Encoder: DPX / H.264 / QuickTime |
| `File > Export > Render Video` | Menu | — | Image sequence / QuickTime alternative |
| `Edit > Preferences` / panel menu | Preference | — | `Enable Timeline Shortcut Keys` gates the keymap |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Panel mode | enum | Frame animation | Frame animation / Video timeline | Convertible both ways |
| Duration | seconds | 10 s | ≥ frame interval | Document Settings; shrinking trims the tail |
| Frame rate | fps | 30 (non-video preset) | 24 / 23.976 / 25 / 29.97 / custom; web 10–15 | Document Settings |
| Time units | enum | Frame numbers | Frame numbers / Timecode | `Alt`-click current-time to toggle |
| Frame delay | seconds | *(not stated)* | decimals (e.g. 0.25) | Frame mode; default not in fetched text (Open questions) |
| Looping | enum | *(not stated)* | Once / 3 Times / Forever / Other | Default not confirmed in text (Open questions) |
| Keyframe interpolation | enum | *(assumed Linear)* | Linear / Hold | "Tweening" in timeline terms; default not stated in fetched text |
| Onion skin count | int | 1 before / 1 after | Frames Before / Frames After | Onion Skin Settings |
| Onion skin spacing | int | 1 | ≥ 1 | Frames between shown frames |
| Onion skin opacity | percent | — | Max Opacity / Min Opacity | Distinguishes neighbouring frames |
| Onion skin blend mode | enum | Normal | layer blend modes | Overlap appearance |
| Frame disposal (GIF) | enum | Automatic | Automatic / Do Not Dispose / Dispose | Frame mode export |
| Clip speed | percent | 100% | interacts with Duration (400% ⇒ ¼ max duration) | Right-click clip |
| Transition duration | seconds / frames | preset | numeric | Right-click transition to set exactly |
| Audio volume / fade | float / seconds | — | level and fade-in/out lengths *(ranges not stated in fetched text)* | Per audio clip/track |
| Keyframe fields (animatable) | — | — | Position, Opacity, Style, Vector Mask Position/Enable, layer-mask position, 3D object/camera position | Time-Vary stopwatch per property |
| Video-layer bit depths | enum | RGB 8 | Grayscale 8/16/32; RGB 8/16/32; CMYK 8/16; Lab 8/16 | Sourced |

## Algorithms & pipeline

The Timeline panel is a **model-view-controller over time**:

- **Frame animation** stores, per frame, a sparse delta of layer state (visibility, position, opacity, style) plus a duration and disposal method; playback composites the layer stack for the current frame. Tweening linearly interpolates position/opacity/effect parameters across inserted frames.
- **Video timeline** stores, per animatable layer property, a sorted list of `(time, value)` keyframes with an interpolation function; evaluation samples the track at the current time. `Linear` is piecewise-linear; `Hold` is a step function. This is standard keyframe animation and is implementable without any Adobe code.
- **Layer duration bars** define each layer's active interval; trimming clamps the interval without altering keyframes outside it (keyframes past the end are trimmed).
- **Video layers** decode frames from the referenced source on demand; a frame cache feeds the compositing pipeline (`ARCH-006`). Edits are stored as per-frame deltas/altered-video markers, not by rewriting the source file.
- **Audio tracks** decode to PCM and play/mix independently of the visual tracks.

### Linux / FFmpeg mapping considerations

This is the crux of the non-goal decision and its possible revisit:

- **Decode/encode.** FFmpeg is the practical Linux media engine and is already the default backend of Qt 6 Multimedia. Options in Rust are `ffmpeg-next` (safe bindings over FFmpeg) or GStreamer via `gstreamer-rs`. FFmpeg covers the CS6 import list and H.264/DPX/MOV export; it does **not** reproduce Adobe Media Encoder's exact presets.
- **Playback.** Qt 6 Multimedia provides `QMediaPlayer` + `QAudioOutput` + `QVideoSink`/`QVideoWidget` with the FFmpeg backend (or GStreamer on embedded Linux). `QVideoSink` is the hook for pulling decoded frames into the compositor instead of a second video widget.
- **Codec/patent caveats.** Qt's own documentation warns that H.264 is patent-encumbered regardless of implementation; packaging must account for it. Legacy Codecs (DV, FLV/F4V, QuickTime MOV) may be unavailable or only partially supported by the chosen FFmpeg build.
- **Keyframe model.** Qt Quick Timeline (`qtquicktimeline`) provides declarative keyframe animation for QML, but the document's keyframes belong to the Rust document model, not the UI; the Qt layer should render and edit them while Rust evaluates them.
- **GIF export.** Frame mode's disposal/looping semantics map to the GIF spec; a Rust GIF encoder (e.g. the `image`/`gif` ecosystem) must honour dispose/blend methods.
- **Source-file links.** CS6 video layers reference the original file; the FFmpeg path should match that (store a path + relink/replace-footage flow) or embed the media, and must decide which.

Mark all of this as a **proposal**; none of it reproduces CS6 timeline parity.

## Rust module mapping

- `pictura_anim::Timeline` — document-level `{ mode: Frame | Video, duration, frame_rate, tracks }`.
- `pictura_anim::FrameAnimation` — `Vec<Frame>` with `{ delay, disposal, layer_state_delta }`.
- `pictura_anim::KeyframeTrack<T>` — sorted `(time, value)` + interpolation; `sample(t) -> T` (linear/hold).
- `pictura_anim::property::{PropertyId, PropertyValue}` — Position, Opacity, Style, VectorMask, layer-mask, 3D object/camera.
- `pictura_anim::tween` — frame-mode tweening (insert interpolated frames between two frames).
- `pictura_anim::video_layer` — `{ source: MediaRef, frame_cache, altered_frames }`; `MediaRef` is a path or embedded handle.
- `pictura_anim::audio` — `{ tracks: Vec<AudioTrack>, clips, volume, fades }`; PCM buffers via the media backend.
- `pictura_media` — decode/encode trait behind which FFmpeg (`ffmpeg-next`) or GStreamer lives; `decode_frame(t) -> Tile`, `encode(stream, options)`.
- `pictura_anim::onion` — onion-skin settings evaluation.
- `pictura_core::Edition` — no longer gates the Timeline in CS6 (Core), but gates 3D property tracks.

Data crossing the boundary: `Time` (rational or microseconds), `PropertyId`, `PropertyValue`, `FrameId`, `TrackId`, decoded frame tiles. No Qt media types (`QImage`/`QVideoFrame`) leak into the Rust model.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `TimelinePanel` | `QDockWidget` | Host; mode switch, ruler, tracks, playback |
| `TimeRuler` | `QWidget` | Ruler, ticks, current-time indicator, zoom |
| `TrackHeaderView` | `QHeaderView`/`QWidget` | Layer names, favourite filter, Time-Vary stopwatch, navigator |
| `KeyframeView` | `QWidget`/`QGraphicsView` | Keyframe diamonds/rects, drag, select, context menu (Linear/Hold) |
| `DurationBarDelegate` | `QStyledItemDelegate` | Layer duration bars, trim handles, work area |
| `AudioTrackWidget` | `QWidget` | Waveform, mute/volume/fade, track menus |
| `TransitionEditor` | `QWidget` | Transition blocks and numeric duration |
| `CommentTrack` | `QWidget`/`QListView` | Timeline comments + HTML export |
| `FrameStripView` | `QListView` | Frame-mode thumbnails, delay, disposal |
| `OnionSkinSettingsDialog` | `QDialog` | Count/spacing/opacity/blend |
| `MediaPlayerAdapter` | `QMediaPlayer` + `QAudioOutput` + `QVideoSink` | Playback and frame capture (FFmpeg/GStreamer backend) |
| `ExportVideoDialog` | `QDialog` | Render Video: format/preset, DPX/H.264/MOV, image sequence |
| `PlaybackControls` | `QToolBar` | Spacebar play, first/prev/next/last, loop |

Widgets over QML for the dense timeline chrome; `QVideoSink` rather than `QVideoWidget` keeps the canvas under the Rust compositor's control (`ARCH-006`). If a QML timeline is preferred, Qt Quick Timeline is available but must not become the source of truth for keyframes.

## Data-model impact

- **Timeline is document data.** Duration, frame rate, mode, per-property keyframes, frame delays/disposal, layer duration intervals, transitions, comments, and work areas are saved in the PSD and referenced by other Adobe apps.
- **Video layers** are a layer kind referencing external media; a `Core` build must round-trip them opaquely and report the media as unsupported if it cannot decode it (`OVR-002`/`OVR-003`).
- **Undo granularity:** one history state per timeline edit — keyframe add/move/delete, clip move/trim/split, transition change, frame add/delete/reorder, tween, audio edit. Live drags coalesce; playback creates no states.
- **Smart Object conversion** is a layer-level change with its own history state and it wraps the video layer for Smart Filters.
- **PSD/XMP keys** for the timeline are not asserted here; exact keys require the format spec (Open questions).
- **Preferences** (`Enable Timeline Shortcut Keys`, panel options, onion-skin defaults) are not document data.

## Edge cases

- **Standard vs Extended.** Timeline is `Core` in CS6, so it must be present in both editions; do not copy the CS5 Extended-only gate.
- **Frame ↔ video conversion.** Converting video timeline → frame animation can drop interpolated keyframes (CS6 notes the appearance is preserved); the conversion must be lossy-by-design and documented.
- **No decoder for a codec.** Opening a CS6 file with a DV/FLV/F4V layer without the codec must preserve the reference and report it unsupported, not corrupt or drop it.
- **Source media moved/deleted.** Provide relink/replace-footage (CS6 requires the original path to stay valid).
- **Background layer.** Cannot be animated; frame/timeline operations must prompt or skip it.
- **Empty / 1-frame documents.** Timeline math and trimming must handle the minimum duration and no keyframes.
- **32-bit / CMYK / Lab video layers.** Restricted bpc sets must be enforced on import.
- **Alpha channels.** Straight vs premultiplied video/image sequences with alpha can halo against a background; a matte colour may be required.
- **Huge PSB documents.** Frame caching must respect the memory cap (`ARCH-006`) and evict.
- **Precision.** Time should be stored rationally (or in exact micro/seconds) so 23.976/29.97 fps do not drift; never accumulate floating time.
- **Audio without a sink / headless.** Playback must be optional; editing must not require an audio device.
- **Codec licensing.** H.264 and others may be patent-encumbered in some markets regardless of implementation; packaging must be explicit.
- **Undo across mode switches.** Converting modes is undoable; the corpus must define whether keyframe loss is recoverable.

## Parity acceptance criteria

Tier is `Non-goal` for exact timeline parity; criteria assert structure, isolation, and preservation:

1. Given either edition, `Window > Timeline` opens the Timeline panel with both frame and video modes.
2. Given a video timeline with two Position keyframes, scrubbing between them moves the layer linearly; setting Hold yields a step.
3. Given frame mode, changing a frame delay and loop option is reflected in an exported animated GIF within the GIF disposal semantics.
4. Given a video layer, converting the document to a Smart Object enables Smart Filters, and a filter applies across frames.
5. Given a CS6 PSD containing timeline/video-layer data, a `Core` build preserves the data on re-save and reports undecodable media as unsupported.
6. Given 29.97 fps, playing two minutes does not drift beyond one frame (rational time model).
7. Given no audio device, timeline editing and export still succeed.
8. Given a moved source file, the app offers relink/replace rather than failing to open the document.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: clip-based Timeline panel and its CS6 redesign; Video Groups; audio tracks and per-clip audio; transitions; duration/speed; Smart Object → Smart Filter workflow for video; motion presets → Transform keyframes; frame-mode controls (loop, delay, tween, duplicate, convert); frame disposal methods; unify/propagate behaviours; copy/paste frame methods; timeline-mode controls (Time-Vary stopwatch, current-time indicator, time ruler, layer duration bar, work-area indicators, keyframe navigator, Global Lighting track, Altered Video track, Comments track, cached frames, onion skin); Linear/Hold interpolation; Document Settings (duration 10 s; 30/25/29.97/24 fps) and import formats (3GP/3G2, AVI, DV, FLV/F4V, MPEG-1/4, MOV, WAV); Adobe Media Encoder export (DPX, H.264, QuickTime); video layers, blank video layers, `Layer > Video Layers`, supported colour modes/bpc; frame-animation export paths (animated GIF, image sequence, QuickTime); timeline shortcut keys and `Enable Timeline Shortcut Keys`; the CS5-era note that video layers are unusable while the panel is in Frame mode.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release: "Intuitive Video Creation" listed among base CS6 features (not Extended-only), alongside the Extended-only 3D items.
- `https://design.tutsplus.com/tutorials/how-to-use-the-new-timeline-panel-in-photoshop-cs6--psd-16515` — CS6 tutorial describing the new video Timeline panel and clip effects.
- `https://doc.qt.io/qt-6/qtmultimedia-index.html` — Qt 6 Multimedia: `QMediaPlayer`, `QAudioOutput`, `QVideoWidget`, `QVideoSink`; FFmpeg 7.1.3 default backend (GStreamer on embedded Linux); `QT_MEDIA_BACKEND` switching; H.264 patent caution.
- `https://docs.rs/ffmpeg-next/latest/ffmpeg_next/` — `ffmpeg-next` crate: safe Rust bindings over FFmpeg (codec/format/filter/software modules), FFmpeg 4-compatible fork.
- `https://prodesigntools.com/products/adobe-cs6-system-requirements.html` — CS6 / Extended requirements page (context for GPU and OS support).

Consulted as search-result snippets only (not individually fetched; community/third-party):

- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+Timeline+panel+video+all+editions+animation+panel+renamed` — results confirming CS6's "new video timeline panel," Frame vs Video Timeline switching, and `Window > Timeline` in later versions.
- SearXNG query for "Photoshop 3D features removed discontinued CC 2015 2021" — surfaced the Adobe 3D FAQ and CG Channel coverage used in `TOOL-060`; not directly about the timeline.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern `video-features` and timeline help pages were inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **"Animation" vs "Timeline" naming drift.** The CS6 Help PDF mostly says "Animation panel" and tags timeline mode "Photoshop Extended," while CS6 marketing/tutorials call it the **Timeline panel** and `OVR-002` records `Window > Timeline` for all editions. *Resolves with:* a CS6 build/capture or the archived CS6 Help web page for "Timeline panel overview."
- **Was video timeline mode truly in Standard, or only the clip-based panel?** `OVR-002` and the Adobe press release say video moved to all editions; the PDF's Extended tags are CS5-era. *Resolves with:* a Standard CS6 capture of `Window > Timeline`.
- **Frame-mode loop default** (Once vs Forever) is not stated in the fetched text. *Resolves with:* a CS6 capture or archived Help page.
- **PSD/XMP serialization of timeline data** (keyframes, frame delays, disposal, comments, work areas) is not asserted here. *Resolves with:* the Adobe PSD/PSB format spec and `01-architecture/file-formats.md`.
- **Do video layers embed or reference media on save?** CS6 references the original file. *Resolves with:* a document-model decision in `ARCH-002` and `10-workflow-io/`.
- **Which codecs/presets the FFmpeg path must support** for import and export, and how patent/licensing is handled. *Resolves with:* `10-workflow-io/export-formats.md` and `01-architecture/build-and-packaging.md`.
- **Does the project implement any timeline at all**, or only a decoder plus an opaque video-layer representation? *Resolves with:* an ADR following `OVR-003`.
- **Rust media backend choice** (`ffmpeg-next` vs `gstreamer-rs` vs both) and its relationship to Qt Multimedia's backend. *Resolves with:* a media-architecture spike in `01-architecture/`.
