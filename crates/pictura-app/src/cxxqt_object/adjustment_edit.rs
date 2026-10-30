//! The Properties panel's adjustment editing: the page descriptor of the
//! adjustment layer at `path`, live parameter / curve / reset edits that
//! recomposite without a history state, and the commit that records one
//! "Modify … Layer" state per gesture. Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The Properties page for the adjustment layer at `path`, one tab-separated row each: `title`, `note`, `groups` (`|`-joined), `curves`, then a `slider` (key, label, group, value, default, min, max, decimals), `check` / `color` (key, label, group, value, default), or `choice` (… options `|`-joined) per control; group -1 shows in every group. Empty for anything else.
        fn adjustment_page(view: &PictureView, path: &QString) -> QStringList;

        /// Set parameter `key` of the adjustment at `path` and recomposite, without a history state. False when refused.
        fn adjustment_set(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            value: f64,
        ) -> bool;

        /// A Curves layer's `channel` (0 composite, 1-3 red, green, blue) points as `"x,y x,y …"`, or empty.
        fn adjustment_curve(view: &PictureView, path: &QString, channel: i32) -> QString;

        /// Replace a Curves layer's `channel` points (`"x,y x,y …"`) and recomposite, without a history state.
        fn adjustment_set_curve(
            view: Pin<&mut PictureView>,
            path: &QString,
            channel: i32,
            points: &QString,
        ) -> bool;

        /// Every parameter of the adjustment at `path` back to its default, without a history state.
        fn adjustment_reset(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Record the edits since the last state as one "Modify `title` Layer" state.
        fn adjustment_commit(view: Pin<&mut PictureView>, title: &QString);

        /// Abandon a live adjustment preview and restore the pre-gesture display.
        fn adjustment_cancel(view: Pin<&mut PictureView>);
    }
}
