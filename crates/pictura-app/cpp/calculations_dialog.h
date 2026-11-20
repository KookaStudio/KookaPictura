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

class PictureView;

// Image > Calculations (#610): blend two individual channels from one or more
// open documents through Blending, Opacity, and an optional Mask, and send the
// result to a new grayscale document, a new alpha channel, or the selection.
// The dialog only gathers settings and previews; the Image menu handler owns
// the document/channel/selection write.
class CalculationsDialog : public QDialog {
    Q_OBJECT

public:
    // One open document, by tab title. Documents of other pixel dimensions are
    // dropped; the active document is listed first.
    struct Source {
        QString name;
        PictureView* view = nullptr;
    };

    enum class Result { NewDocument, NewChannel, Selection };

    CalculationsDialog(PictureView* view, const QList<Source>& documents,
                       QWidget* parent = nullptr);
    ~CalculationsDialog() override;

    Result result() const;

    // Write the current result to `target`: the active document for Selection
    // and New Channel, or a fresh view for New Document.
    bool apply(PictureView* target) const;

    // Refuses a New Channel result at the 56-channel cap and names the limit.
    void accept() override;

private:
    struct Snapshots;
    struct Picker {
        QComboBox* source = nullptr;
        QComboBox* layer = nullptr;
        QComboBox* channel = nullptr;
        QCheckBox* invert = nullptr;
    };

    void buildUi();
    Picker makePicker(const QString& prefix);
    void rebuildLayers(const Picker& picker, bool mask);
    void rebuildChannels(const Picker& picker, bool mask);
    void readSources();
    void updateModeRows();
    void updatePreview();

    PictureView* view_ = nullptr;
    QList<Source> sources_;
    std::unique_ptr<Snapshots> snapshots_;

    Picker source1_;
    Picker source2_;
    Picker mask_;
    QComboBox* blending_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QLabel* scaleLabel_ = nullptr;
    QDoubleSpinBox* scale_ = nullptr;
    QLabel* offsetLabel_ = nullptr;
    QSpinBox* offset_ = nullptr;
    QCheckBox* useMask_ = nullptr;
    QGroupBox* maskGroup_ = nullptr;
    QComboBox* result_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QLabel* previewImage_ = nullptr;
    QLabel* warning_ = nullptr;
};

} // namespace pictura
