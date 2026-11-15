#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;

namespace pictura {

/// The Custom proof setup values: the device to simulate and its options.
struct ProofSetupValues {
    int device = 0;  // 0 Working CMYK, 1 Working Gray, 2 sRGB, 3 Adobe RGB, 4 ProPhoto
    int intent = 1;  // 0 Perceptual, 1 Relative, 2 Saturation, 3 Absolute
    bool bpc = true;
    bool simulatePaper = false;
    bool simulateBlack = false;
};

/// View > Proof Setup > Custom: Device to Simulate, Rendering Intent, Black
/// Point Compensation, and the paper/black-ink simulations.
class ProofSetupDialog : public QDialog {
    Q_OBJECT

public:
    explicit ProofSetupDialog(const ProofSetupValues& values, QWidget* parent = nullptr);

    ProofSetupValues values() const;

    // Test hooks.
    void setDeviceForTest(int device);
    void setIntentForTest(int intent);
    void setSimulateBlackForTest(bool on);

private:
    QComboBox* device_ = nullptr;
    QComboBox* intent_ = nullptr;
    QCheckBox* bpc_ = nullptr;
    QCheckBox* paper_ = nullptr;
    QCheckBox* black_ = nullptr;
};

} // namespace pictura
