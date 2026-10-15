#pragma once

// CS6's Reduce Noise dialog: Basic / Advanced pages. Basic sets one Strength
// and Preserve Details for every channel; Advanced picks a Channel and sets
// that channel's pair. Reduce Color Noise, Sharpen Details and Remove JPEG
// Artifacts are global and live on the Basic page. The parameters are a run of
// slots, but the Advanced page rebinds its two sliders to the selected
// channel, so it does not fit the generic FilterPreviewDialog.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "filter_commands.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QTabWidget;

namespace pictura {

class FilterParamControls;
class PictureView;

class ReduceNoiseDialog : public QDialog {
    Q_OBJECT

public:
    ReduceNoiseDialog(PictureView* view, const FilterCommandSpec& spec,
                      QWidget* parent = nullptr);

    // The `reduce-noise` slots, in the mapping order.
    QList<double> values() const;
    // Silent: emits no preview.
    void setValues(const QList<double>& values);

    // Run the dialog modally, committing `out` on OK. On cancel the canvas
    // preview is discarded.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const QList<double>& initial, QList<double>* out, QWidget* parent);

protected:
    void showEvent(QShowEvent* event) override;

private:
    // The per-channel store index (0, 2 or 4) for the loaded channel.
    int channelSlot() const;
    // Store the Advanced sliders into the loaded channel, then load the
    // channel the combo now selects.
    void reloadChannel();
    void storeChannel();
    void loadChannel();
    void valuesChanged(bool canvas);
    void renderPad();
    void discardPreview();

    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    FilterParamControls* basic_ = nullptr;
    FilterParamControls* advanced_ = nullptr;
    QComboBox* channel_ = nullptr;
    QTabWidget* tabs_ = nullptr;
    QLabel* pad_ = nullptr;
    QCheckBox* preview_ = nullptr;
    // Slots 6..11: the per-channel Strength/Preserve pairs for R, G and B.
    QList<double> perChannel_;
    int loadedChannel_ = 0;
    QImage proxy_;
    QByteArray proxyRgba_;
    bool previewShown_ = false;
    bool shownOnce_ = false;
};

} // namespace pictura
