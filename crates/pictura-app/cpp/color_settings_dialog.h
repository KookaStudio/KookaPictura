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

// The Color Settings dialog: shows the fixed sRGB RGB working space and edits
// the incoming-profile policy. The combo item data is the persistence code:
// 0 Preserve Embedded Profiles, 1 Convert to Working RGB, 2 Off. More Options
// reveals Advanced Controls: Blend Text Colors Using Gamma (on, 1.45 in CS6).
// ponytail: sRGB working space only; no .csf file or CMYK/Gray policies, and
// Blend Text Colors Using Gamma is the only advanced control.
class ColorSettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit ColorSettingsDialog(int currentPolicy, QWidget* parent = nullptr);

    int policyCode() const;

    void setTextGamma(bool enabled, double gamma);
    bool textGammaEnabled() const;
    double textGamma() const;

    // Test hooks.
    QStringList policyChoicesForTest() const;
    QString workingSpaceForTest() const;
    void setPolicyForTest(int index);

private:
    QComboBox* combo_ = nullptr;
    QLabel* workingSpace_ = nullptr;
    QPushButton* moreOptions_ = nullptr;
    QGroupBox* advanced_ = nullptr;
    QCheckBox* blendTextGamma_ = nullptr;
    QDoubleSpinBox* textGammaValue_ = nullptr;
};

} // namespace pictura
