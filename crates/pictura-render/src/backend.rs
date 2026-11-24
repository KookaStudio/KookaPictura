//! The compositing backend that produced a buffer.
//!
//! Unconditional so the CPU composite API (`CompositeRequest`, `composite`,
//! `DocumentComposite`) reports its backend in a build without the `gpu`
//! feature. The `gpu` module re-exports it, which is why
//! `pictura_render::gpu::Backend` keeps resolving.

/// The compositing backend that produced a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// The wgpu compute compositor on the selected backend.
    Gpu,
    /// The CPU oracle [`crate::composite_rgba`].
    Cpu,
}
