#pragma once

#include <QtCore/QRect>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QTimer;

namespace pictura {

class HdrToningControls;
class PictureView;

// Image > Adjustments > HDR Toning (Local Adaptation). A preset combo loads the
// 17 CS6 presets into the shared nine-control widget; any edit drops it back to
// Custom. The Preview checkbox shows the result on the canvas while the dialog
// is open, OK applies one "HDR Toning" state, Cancel restores the layer exactly.
class HdrToningDialog : public QDialog {
    Q_OBJECT

public:
    HdrToningDialog(PictureView* view, const QRect& visible, QWidget* parent = nullptr);
    ~HdrToningDialog() override;

    // Run the dialog modally over `view`'s active layer; true when applied.
    static bool get(QWidget* parent, PictureView* view, const QRect& visible);

    // Test hooks.
    int presetCount() const;
    QWidget* controlForTest(const QString& name) const;

    void accept() override;
    void reject() override;

private:
    void loadPreset(int index);
    void edited();
    void preview();
    void cancelPreview();

    PictureView* view_ = nullptr;
    QRect visible_;
    bool previewing_ = false;
    bool loading_ = false;
    // A slider drag edits many times a frame; the preview runs once it pauses.
    QTimer* settle_ = nullptr;

    QComboBox* preset_ = nullptr;
    HdrToningControls* controls_ = nullptr;
    QCheckBox* preview_ = nullptr;
};

} // namespace pictura
