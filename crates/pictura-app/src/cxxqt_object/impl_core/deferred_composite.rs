//! The deferred Move and transform workers (#1157).

mod imp;

pub(in crate::cxxqt_object) use imp::{
    move_composite_pending, move_composite_set_delay, transform_pending, transform_set_delay,
    CompositeJob, DeferredComposite, DeferredTransform, TransformJob, TransformSpec,
};
