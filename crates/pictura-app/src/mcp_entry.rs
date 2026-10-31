//! `pictura --mcp`: hands the process to the stdio bridge. A free function, in
//! its own bridge, because no Qt object exists yet when it runs.

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Serve MCP on stdio until stdin closes, and return the exit code.
        fn mcp_run(socket: &QString, token: &QString, args: &QStringList) -> i32;
    }
}
