#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtWidgets/QDialog>

#include <memory>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QGroupBox;
class QLabel;
class QSpinBox;

namespace pictura {

class ImageView;
class PictureView;

// Image > Apply Image (#609): blend a layer and channel of an open document
// into the active layer or mask, with Invert, Blending, Opacity, Preserve
// Transparency, and an optional Mask. Modal: the source and mask are read once
// when their controls change, so a preview cannot feed back into a source on
// the same document, and OK commits exactly one "Apply Image" state.
class ApplyImageDialog : public QDialog {
    Q_OBJECT

public:
    // One open document, by tab title. Documents of other pixel dimensions
    // are dropped; the active document is listed first.
    struct Source {
        QString name;
        PictureView* view = nullptr;
    };

    ApplyImageDialog(PictureView* view, ImageView* canvas, const QList<Source>& documents,
                     QWidget* parent = nullptr);
    ~ApplyImageDialog() override;

    void accept() override;
    void reject() override;

private:
    struct Snapshots;
    struct Picker {
        QComboBox* source = nullptr;
        QComboBox* layer = nullptr;
        QComboBox* channel = nullptr;
    };

    void buildUi();
    Picker makePicker(const QString& prefix);
    void rebuildLayers(const Picker& picker, bool mask);
    void rebuildChannels(const Picker& picker, bool mask);
    void readSource();
    void readMask();
    void updateModeRows();
    void applyPreview();
    void cancelPreview();

    PictureView* view_ = nullptr;
    ImageView* canvas_ = nullptr;
    QList<Source> sources_;
    std::unique_ptr<Snapshots> snapshots_;
    bool previewing_ = false;

    Picker source_;
    Picker mask_;
    QCheckBox* invert_ = nullptr;
    QComboBox* blending_ = nullptr;
    QLabel* scaleLabel_ = nullptr;
    QDoubleSpinBox* scale_ = nullptr;
    QLabel* offsetLabel_ = nullptr;
    QSpinBox* offset_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QCheckBox* preserve_ = nullptr;
    QCheckBox* useMask_ = nullptr;
    QGroupBox* maskGroup_ = nullptr;
    QCheckBox* maskInvert_ = nullptr;
    QCheckBox* preview_ = nullptr;
};

} // namespace pictura
