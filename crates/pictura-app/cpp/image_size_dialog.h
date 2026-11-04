#pragma once

#include <QtWidgets/QDialog>

#include <functional>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QToolButton;

namespace pictura {

class PictureView;

// A byte count as CS6 writes it in Image Size / Canvas Size: "1.25M", "980.0K".
QString imageSizeSummary(double bytes);
// The bytes one pixel of `view`'s document takes (colour channels × depth).
double bytesPerPixel(PictureView* view);
// Whether a `width` × `height` resample of `view`'s document fits the
// ImageBudget caps bounded by the memory available now (#1156).
bool imageSizeFitsBudget(PictureView& view, int width, int height);
// The warning the Image Size handler shows when it does not.
QString imageSizeBudgetMessage(int width, int height);
// Test seam: answer the Image Size dialog and its budget warning.
void setImageSizeDialogDriverForTest(std::function<bool(QDialog&)> driver);
// Run `dialog` under the test driver, or modally; true when accepted.
bool answerImageSizeDialog(QDialog& dialog, QWidget* parent);
// Show the budget warning through the test driver, or modally.
void showImageSizeBudgetWarning(QWidget* parent, int width, int height);

// Image > Image Size. Width, Height, and Resolution are three views of a pixel
// count and a pixels-per-inch figure. With Resample on, Width and Height set
// the pixel count and Resolution alone changes no pixels; with it off the
// pixels are fixed, so Width, Height, and Resolution move together and only the
// print size changes. Fit To offers CS6's screen and print presets (a
// landscape image takes a preset on its side). Ported from photorust's
// ImageSizeDialog.
// ponytail: Auto Resolution is listed but disabled; Automatic, Preserve
// Details, and the Bicubic Smoother/Sharper entries all resample bicubic.
class ImageSizeDialog : public QDialog {
    Q_OBJECT

public:
    explicit ImageSizeDialog(PictureView* view, QWidget* parent = nullptr);

    int resultWidth() const { return pixelWidth_; }
    int resultHeight() const { return pixelHeight_; }
    double resultPpi() const { return resolution_; }
    bool resultPerCm() const;
    // The engine resample kind ("bicubic", "nearest", "bilinear"), or empty
    // with Resample off.
    QString resampleKind() const;

    QWidget* controlForTest(const QString& name) const;

private:
    void buildUi();
    // Recompute every field from the pixel size and resolution.
    void syncFields();
    void widthEdited();
    void heightEdited();
    void resolutionEdited();
    void fitToChosen(int index);
    void applyFitPreset(int preset);
    void markCustom();
    void resampleToggled(bool on);
    void updateSummary();
    // Pixels per unit of Width/Height unit `unit`.
    double unitScale(int unit) const;

    PictureView* view_ = nullptr;
    int pixelWidth_ = 1;
    int pixelHeight_ = 1;
    double resolution_ = 72.0;
    int originalWidth_ = 1;
    int originalHeight_ = 1;
    double originalResolution_ = 72.0;
    double bytesPerPixel_ = 3.0;
    bool updating_ = false;

    QLabel* summary_ = nullptr;
    QLabel* dimensions_ = nullptr;
    QLabel* preview_ = nullptr;
    QComboBox* fitTo_ = nullptr;
    QDoubleSpinBox* width_ = nullptr;
    QDoubleSpinBox* height_ = nullptr;
    QDoubleSpinBox* resolutionField_ = nullptr;
    QComboBox* widthUnit_ = nullptr;
    QComboBox* heightUnit_ = nullptr;
    QComboBox* resolutionUnit_ = nullptr;
    QCheckBox* resample_ = nullptr;
    QComboBox* resampleMode_ = nullptr;
    QToolButton* chain_ = nullptr;
};

} // namespace pictura
