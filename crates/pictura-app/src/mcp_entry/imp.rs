use cxx_qt_lib::{QList, QString, QStringList};

pub(super) fn mcp_run(socket: &QString, token_path: &QString, args: &QStringList) -> i32 {
    let args: Vec<String> = QList::<QString>::from(args)
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let endpoint = pictura_mcp::Endpoint {
        socket: socket.to_string(),
        token_path: token_path.to_string(),
    };
    pictura_mcp::run(endpoint, &args)
}
