#pragma once

// CS6's Picture Frame dialog: a frame thumbnail picker, a preview of the
// picture with the frame drawn on it, and Basic / Advanced option pages. The
// parameters are a run of slots but the frame is chosen from a picker, so it
// does not fit the generic FilterPreviewDialog.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "filter_commands.h"

class QCheckBox;
class QLabel;
class QListWidget;

namespace pictura {

class FilterParamControls;
class PictureView;

class PictureFrameDialog : public QDialog {
    Q_OBJECT

public:
    PictureFrameDialog(PictureView* view, const FilterCommandSpec& spec,
                       QWidget* parent = nullptr);

    // The `picture-frame` slots, in the mapping order.
    QList<double> values() const;
    // Silent: emits no preview.
    void setValues(const QList<double>& values);

    // The pad's picture: the proxy with the current frame drawn on it.
    QImage padImage() const;

    // Run the dialog modally, committing `out` on OK. On cancel the canvas
    // preview is discarded.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const QList<double>& initial, QList<double>* out, QWidget* parent);

protected:
    void showEvent(QShowEvent* event) override;

private:
    void buildPicker();
    void renderPad();
    void valuesChanged(bool canvas);
    void discardPreview();
    void setFrame(int frame);

    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    QListWidget* picker_ = nullptr;
    QLabel* pad_ = nullptr;
    FilterParamControls* basic_ = nullptr;
    FilterParamControls* advanced_ = nullptr;
    QCheckBox* preview_ = nullptr;
    // The picture shrunk to the pad, taken before any canvas preview so the
    // frame is never drawn twice.
    QImage proxy_;
    QByteArray proxyRgba_;
    bool previewShown_ = false;
    bool shownOnce_ = false;
};

} // namespace pictura
