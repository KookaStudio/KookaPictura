#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QGroupBox;
class QLabel;
class QPushButton;

namespace pictura {

/// The Color Settings values the frame persists in `SessionState`.
struct ColorSettingsValues {
    int rgbPolicy = 0;              // 0 Preserve, 1 Convert, 2 Off
    int cmykPolicy = 0;             // 0 Preserve, 1 Convert, 2 Off
    int workingCmyk = 0;            // 0 Photocraft Coated CMYK
    int renderIntent = 1;           // 0 Perceptual, 1 Relative, 2 Saturation, 3 Absolute
    bool blackPointCompensation = true;
    bool useDither = true;
    bool askMismatchOpen = false;
    bool askMissingOpen = false;
    bool textGammaEnabled = true;
    double textGamma = 1.45;
};

// The Color Settings dialog: fixed sRGB RGB working space, the bundled CMYK
// working space, RGB/CMYK incoming profiles, warning asks, conversion options,
// and the existing Advanced Controls text gamma.
class ColorSettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit ColorSettingsDialog(const ColorSettingsValues& values,
                                 QWidget* parent = nullptr);
    explicit ColorSettingsDialog(int currentPolicy, QWidget* parent = nullptr);

    ColorSettingsValues values() const;

    int policyCode() const;
    int cmykPolicyCode() const;
    int workingCmyk() const;
    int renderIntent() const;
    bool blackPointCompensation() const;
    bool useDither() const;
    bool askMismatchOpen() const;
    bool askMissingOpen() const;

    void setTextGamma(bool enabled, double gamma);
    bool textGammaEnabled() const;
    double textGamma() const;

    // Test hooks.
    QStringList policyChoicesForTest() const;
    QStringList cmykPolicyChoicesForTest() const;
    QStringList intentChoicesForTest() const;
    QString workingSpaceForTest() const;
    QString cmykSpaceForTest() const;
    void setPolicyForTest(int index);
    void setCmykPolicyForTest(int index);
    void setIntentForTest(int code);
    void setAskMismatchForTest(bool on);
    void setAskMissingForTest(bool on);

private:
    void buildUi(const ColorSettingsValues& values);

    QComboBox* rgbPolicy_ = nullptr;
    QComboBox* cmykPolicy_ = nullptr;
    QComboBox* cmykSpace_ = nullptr;
    QComboBox* intent_ = nullptr;
    QLabel* workingSpace_ = nullptr;
    QCheckBox* bpc_ = nullptr;
    QCheckBox* dither_ = nullptr;
    QCheckBox* askMismatch_ = nullptr;
    QCheckBox* askMissing_ = nullptr;
    QPushButton* moreOptions_ = nullptr;
    QGroupBox* advanced_ = nullptr;
    QCheckBox* blendTextGamma_ = nullptr;
    QDoubleSpinBox* textGammaValue_ = nullptr;
};

} // namespace pictura
