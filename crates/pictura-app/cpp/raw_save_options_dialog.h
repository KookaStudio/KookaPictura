#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

#include <functional>

class QCheckBox;
class QComboBox;
class QLineEdit;

namespace pictura {

class PictureView;

// Save Options for a raw open's Save Image (#402): the destination folder and
// file name, and the DNG options Compatibility, JPEG Preview, Embed Original
// Raw File and Convert to Linear Image.
class RawSaveOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit RawSaveOptionsDialog(const QString& defaultPath, QWidget* parent = nullptr);

    // The chosen folder and name, `.dng` appended when the name lacks it;
    // empty without a name.
    QString targetPath() const;
    // 0–4: Camera Raw 2.4 … 6.6 and later.
    int compatibility() const;
    // 0 None, 1 Medium Size, 2 Full Size.
    int preview() const;
    bool embedOriginal() const;
    bool linear() const;

private:
    QLineEdit* folder_ = nullptr;
    QLineEdit* name_ = nullptr;
    QComboBox* compatibility_ = nullptr;
    QComboBox* preview_ = nullptr;
    QCheckBox* embedOriginal_ = nullptr;
    QCheckBox* linear_ = nullptr;
};

// Runs Save Image for `view`'s pending raw with `settings` (Pictura Raw text):
// Save Options, the replace question for an existing file, then the save on
// a worker under a cancellable progress window. Nested dialogs go through
// `runDialog` (true when accepted; a message box is answered by its clicked
// button). Returns the message to show, empty when nothing was attempted.
QString runRawSaveImage(QWidget* parent, PictureView* view, const QString& settings,
                        const std::function<bool(QDialog&)>& runDialog);

} // namespace pictura
