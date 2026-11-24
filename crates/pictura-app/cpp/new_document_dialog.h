#pragma once

#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>

class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QLineEdit;
class QSpinBox;

namespace pictura {

class ColorSwatchWidget;

// The values collected by the New Document dialog.
struct NewDocumentSpec {
    QString name;
    int width = 1280;
    int height = 800;
    QString mode = QStringLiteral("rgb"); // "bitmap" | "grayscale" | "rgb" | "cmyk" | "lab"
    int depth = 8;                        // 1 (Bitmap only), 8, 16, or 32
    QColor fill = Qt::white;              // the layer's colour unless `transparent`
    bool transparent = false;
    double ppi = 72.0;
    bool perCm = false;
};

// File > New in CS6's layout: Name, Document Type and its Size presets,
// Width/Height with their unit, Resolution, Color Mode with bit depth,
// Background Contents, the Advanced section, and OK / Cancel / Save Preset /
// Delete Preset with the Image Size estimate down the right. Editing a size
// turns the Document Type to Custom; a mode offers only the depths it takes.
// Ported from photorust's NewDocumentDialog.
// ponytail: Save / Delete Preset, Color Profile (new documents are untagged
// sRGB), and Pixel Aspect Ratio (square only) are shown but disabled.
class NewDocumentDialog : public QDialog {
    Q_OBJECT

public:
    // `name` is the suggested Untitled-n; `background` the toolbox
    // Background colour the Background Contents menu offers.
    NewDocumentDialog(const QString& name, const QColor& background, QWidget* parent = nullptr);

    NewDocumentSpec spec() const;
    int widthPixels() const;
    int heightPixels() const;
    QWidget* controlForTest(const QString& name) const;

    void accept() override;

    // Run the dialog modally; returns true and fills `out` on OK.
    static bool get(QWidget* parent, const QString& name, const QColor& background,
                    NewDocumentSpec* out);

private:
    void buildUi();
    void documentTypeChosen(int index);
    void sizeChosen(int index);
    // Restate Width/Height in the unit just picked, keeping their pixels.
    void unitChosen(int unit);
    void resolutionUnitChosen(int unit);
    void modeChosen();
    void contentsChosen();
    void pickBackgroundColor();
    void updateBackgroundSwatch();
    void dimensionEdited();
    void setPixels(double width, double height, int unit);
    void updateImageSize();
    double pixelsPerUnit(int unit) const;
    double ppi() const;

    QColor background_;
    QColor custom_ = Qt::white;
    bool updating_ = false;
    bool clipboardHasImage_ = false;
    int unit_ = 0;
    // The size in pixels; the fields show it in `unit_`, rounded.
    double pixelWidth_ = 1280.0;
    double pixelHeight_ = 800.0;

    QLineEdit* name_ = nullptr;
    QComboBox* documentType_ = nullptr;
    QComboBox* size_ = nullptr;
    QDoubleSpinBox* width_ = nullptr;
    QDoubleSpinBox* height_ = nullptr;
    QComboBox* unitCombo_ = nullptr;
    QDoubleSpinBox* resolution_ = nullptr;
    QComboBox* resolutionUnit_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* depth_ = nullptr;
    QComboBox* contents_ = nullptr;
    ColorSwatchWidget* backgroundSwatch_ = nullptr;
    QLabel* imageSize_ = nullptr;
};

} // namespace pictura
