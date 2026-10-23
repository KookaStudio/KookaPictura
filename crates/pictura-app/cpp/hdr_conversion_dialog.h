#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QComboBox;
class QDoubleSpinBox;
class QStackedWidget;

namespace pictura {

class HdrToningControls;

// The four CS6 HDR Conversion methods, in dialog order.
enum class HdrConversionMethod {
    LocalAdaptation = 0,
    EqualizeHistogram = 1,
    ExposureGamma = 2,
    HighlightCompression = 3,
};

// The values collected by the HDR Conversion dialog.
struct HdrConversionSpec {
    HdrConversionMethod method = HdrConversionMethod::LocalAdaptation;
    double exposure_ev = 0.0;
    double gamma = 1.0;
    double radius = 187.0;
    double strength = 4.0;
    double local_gamma = 0.99;
    double local_exposure = 0.0;
    int detail = 30;
    int shadow = 0;
    int highlight = 0;
    int vibrance = 0;
    int saturation = 20;
};

// CS6 `Image > Mode > 32 -> 16/8` HDR Conversion. The Method control gates the
// Local Adaptation numeric rows, the Exposure/Gamma fields, or no fields for the
// two automatic methods.
class HdrConversionDialog : public QDialog {
    Q_OBJECT

public:
    explicit HdrConversionDialog(QWidget* parent = nullptr);

    HdrConversionSpec spec() const;

    // Run the dialog modally; returns true and fills `out` on OK.
    static bool get(QWidget* parent, HdrConversionSpec* out);

    // Test hook: the Method combo, the stacked pages, or any local control.
    QWidget* controlForTest(const QString& name) const;

private:
    QComboBox* method_ = nullptr;
    QStackedWidget* pages_ = nullptr;
    HdrToningControls* local_ = nullptr;
    QDoubleSpinBox* exposureSpin_ = nullptr;
    QDoubleSpinBox* gammaSpin_ = nullptr;
};

} // namespace pictura
