#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QGroupBox;
class QLabel;

namespace pictura {

// The built-in-profile chooser for Edit > Assign Profile… / Convert to
// Profile…. The bridge index is the combo item's data: 0 sRGB, 1 Adobe RGB,
// 2 Pro Photo RGB. Convert also exposes the conversion options, flatten, and
// preview.
class ProfileDialog : public QDialog {
    Q_OBJECT

public:
    explicit ProfileDialog(bool convert, QWidget* parent = nullptr);
    ProfileDialog(bool convert, const QString& sourceSpace, QWidget* parent = nullptr);

    int profileIndex() const;
    int renderIntent() const;          // 0 Perceptual, 1 Relative, 2 Saturation, 3 Absolute
    bool blackPointCompensation() const;
    bool useDither() const;
    bool flatten() const;
    bool preview() const;

    void setConversionOptions(int intent, bool bpc, bool dither);

    // Test hooks.
    QStringList choicesForTest() const;
    QStringList intentChoicesForTest() const;
    void setChoiceForTest(int index);
    void setPreviewForTest(bool on);

signals:
    void optionsChanged();

private:
    void buildUi(bool convert, const QString& sourceSpace);

    bool convert_ = false;
    QComboBox* combo_ = nullptr;
    QComboBox* intent_ = nullptr;
    QCheckBox* bpc_ = nullptr;
    QCheckBox* dither_ = nullptr;
    QCheckBox* flatten_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QGroupBox* options_ = nullptr;
    QLabel* source_ = nullptr;
};

} // namespace pictura
