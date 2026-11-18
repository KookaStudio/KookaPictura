//! Batch questions answered from the live library.

use std::path::Path;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_script::batch::{
    self, error_log_line, override_save_path, resolve_folder, BatchDate, BatchOptions, FileNaming,
    TOKEN_LABELS,
};

impl super::ffi::ActionsEngine {
    pub fn batch_validate(&self, options_json: &QString) -> QString {
        let options = match serde_json::from_str::<BatchOptions>(&options_json.to_string()) {
            Ok(options) => options,
            Err(err) => return QString::from(format!("Invalid batch options: {err}").as_str()),
        };
        match batch::validate(&options, &self.rust().library) {
            Ok(()) => QString::default(),
            Err(err) => QString::from(err.to_string().as_str()),
        }
    }

    pub fn action_reaches_command(
        &self,
        set: &QString,
        action: &QString,
        command: &QString,
    ) -> bool {
        let library = &self.rust().library;
        batch::find_action(library, &set.to_string(), &action.to_string()).is_some_and(
            |(set, action)| {
                batch::action_reaches_command(library, set, action, &command.to_string())
            },
        )
    }
}

fn to_qstringlist<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> QStringList {
    let mut list = QList::<QString>::default();
    for item in items {
        list.append(QString::from(item.as_ref()));
    }
    QStringList::from(&list)
}

fn date(year: i32, month: i32, day: i32) -> BatchDate {
    BatchDate {
        year,
        month: month.clamp(1, 12) as u32,
        day: day.clamp(1, 31) as u32,
    }
}

fn naming(json: &QString) -> FileNaming {
    serde_json::from_str(&json.to_string()).unwrap_or_default()
}

pub(super) fn batch_resolve_folder(
    root: &QString,
    recurse: bool,
    extensions: &QStringList,
) -> QStringList {
    let extensions: Vec<String> = extensions
        .iter()
        .map(|ext| ext.to_string().trim_start_matches('.').to_lowercase())
        .collect();
    let paths = resolve_folder(Path::new(&root.to_string()), recurse, &extensions);
    to_qstringlist(paths.iter().map(|path| path.to_string_lossy().into_owned()))
}

pub(super) fn batch_token_labels() -> QStringList {
    to_qstringlist(TOKEN_LABELS.iter())
}

pub(super) fn batch_render_name(
    naming_json: &QString,
    input_name: &QString,
    ext: &QString,
    position: i32,
    year: i32,
    month: i32,
    day: i32,
) -> QString {
    let name = naming(naming_json).render(
        &input_name.to_string(),
        &ext.to_string(),
        usize::try_from(position).unwrap_or(0),
        date(year, month, day),
    );
    QString::from(name.as_str())
}

pub(super) fn batch_naming_has_unique_field(naming_json: &QString) -> bool {
    naming(naming_json).has_unique_field()
}

pub(super) fn batch_override_save_path(
    options_json: &QString,
    input: &QString,
    ext: &QString,
    position: i32,
    year: i32,
    month: i32,
    day: i32,
) -> QString {
    let Ok(options) = serde_json::from_str::<BatchOptions>(&options_json.to_string()) else {
        return QString::default();
    };
    override_save_path(
        &options,
        Path::new(&input.to_string()),
        &ext.to_string(),
        usize::try_from(position).unwrap_or(0),
        date(year, month, day),
    )
    .map(|path| QString::from(path.to_string_lossy().as_ref()))
    .unwrap_or_default()
}

pub(super) fn batch_error_log_line(input: &QString, message: &QString) -> QString {
    QString::from(error_log_line(&input.to_string(), &message.to_string()).as_str())
}
