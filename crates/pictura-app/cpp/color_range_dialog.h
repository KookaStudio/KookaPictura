#pragma once

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QHideEvent;
class QKeyEvent;
class QLabel;
class QRadioButton;
class QSlider;
class QSpinBox;
class QToolButton;

namespace pictura {

class PictureView;
class ToolController;

// Select > Color Range (SEL-005): sample one or more colours, a hue band, a
// tonal band, Skin Tones, or Out Of Gamut; the preview is the coverage mask OK
// will select (intersected with a live selection), and the image window can
// show the Selection Preview modes. Non-modal, so its eyedroppers can reach the
// canvas. ponytail: no Detect Faces, Alt-click Reset, or `.axt` import.
class ColorRangeDialog : public QDialog {
    Q_OBJECT

public:
    ColorRangeDialog(PictureView* view, ToolController* tools, QWidget* parent = nullptr);
    ~ColorRangeDialog() override;

    // 0 Sampled Colors … 10 Shadows, 11 Out Of Gamut (the engine's numbering).
    PictureView* view() const { return view_; }
    int range() const;
    QColor sampledColor() const;
    QString samplesString() const;
    int sampleCount() const { return int(samples_.size()); }
    int fuzziness() const;
    bool localized() const;
    int localizedRange() const;
    bool inverted() const;
    // 0 Selection, 1 Image.
    int displayMode() const;
    // 0 None, 1 Grayscale, 2 Black Matte, 3 White Matte, 4 Quick Mask.
    int selectionPreview() const;

    // Take the composite colour at `imagePos` (the canvas sampler's entry).
    void sampleAt(const QPointF& imagePos, Qt::KeyboardModifiers modifiers = Qt::NoModifier);
    QImage previewForTest() const { return preview_; }
    QImage imagePreviewForTest() const { return imagePreview_; }

    // The settings-file helpers the file dialog and the Qt Test share.
    bool saveSettings(const QString& path);
    bool loadSettings(const QString& path);
    QString settingsError() const { return settingsError_; }

protected:
    void hideEvent(QHideEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;

private:
    struct Sample {
        QPoint pos;
        QColor color;
    };
    enum class PickMode { Replace, Add, Subtract };

    void refreshSampler();
    void refreshPreview();
    void refreshCanvasPreview();
    void refreshEnabled();
    void refreshSwatches();
    void chooseSave();
    void chooseLoad();
    static QString selectNameFor(int index);
    static int indexForSelectName(const QString& name);

    PictureView* view_ = nullptr;
    ToolController* tools_ = nullptr;
    QVector<Sample> samples_;
    PickMode mode_ = PickMode::Replace;

    QComboBox* select_ = nullptr;
    QSpinBox* fuzziness_ = nullptr;
    QSlider* fuzzinessSlider_ = nullptr;
    QLabel* swatch_ = nullptr;
    QToolButton* replaceButton_ = nullptr;
    QToolButton* addButton_ = nullptr;
    QToolButton* subtractButton_ = nullptr;
    QCheckBox* localized_ = nullptr;
    QSpinBox* range_ = nullptr;
    QSlider* rangeSlider_ = nullptr;
    QCheckBox* invert_ = nullptr;
    QRadioButton* selectionButton_ = nullptr;
    QRadioButton* imageButton_ = nullptr;
    QComboBox* previewMode_ = nullptr;
    QLabel* previewLabel_ = nullptr;
    QLabel* status_ = nullptr;
    QImage preview_;
    QImage imagePreview_;
    QString settingsError_;
};

} // namespace pictura
