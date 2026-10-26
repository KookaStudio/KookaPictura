#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QRadioButton;
class QSlider;
class QSpinBox;
class QTimer;

namespace pictura {

class PictureView;

// JPEG Options (WF-1): Matte, Image Options (quality 0-12 with a preset list),
// Format Options (Baseline / Baseline Optimized / Progressive), Scans, Preview,
// and a file-size readout. Quality, format, and matte reach the encoder; the
// scan count is shown disabled (Qt writes its own progressive scan plan).
class JpegOptionsDialog : public QDialog {
    Q_OBJECT

public:
    JpegOptionsDialog(const SaveFormatOptions& initial, PictureView* view,
                      const QColor& foreground, const QColor& background,
                      QWidget* parent = nullptr);

    int quality() const;
    int format() const; // 0 Baseline, 1 Baseline Optimized, 2 Progressive
    int scans() const;
    int matte() const;  // packed 0xRRGGBB, -1 = None
    bool preview() const;

    // Test hooks.
    QString sizeTextForTest() const;
    qint64 estimatedBytesForTest() const { return lastSize_; }
    bool previewPaneVisibleForTest() const;
    void refreshEstimateForTest();

private:
    void syncQualityControls(int value);
    void applyPreset(int index);
    void scheduleEstimate();
    void refreshEstimate();
    int matteColour() const;

    PictureView* view_ = nullptr;
    QColor foreground_;
    QColor background_;
    QColor otherMatte_{Qt::white};

    QComboBox* matte_ = nullptr;
    QComboBox* preset_ = nullptr;
    QSpinBox* quality_ = nullptr;
    QSlider* slider_ = nullptr;
    QRadioButton* baseline_ = nullptr;
    QRadioButton* baselineOptimized_ = nullptr;
    QRadioButton* progressive_ = nullptr;
    QSpinBox* scans_ = nullptr;
    QCheckBox* previewBox_ = nullptr;
    QLabel* size_ = nullptr;
    QLabel* previewImage_ = nullptr;
    QTimer* debounce_ = nullptr;
    qint64 lastSize_ = 0;
};

} // namespace pictura
