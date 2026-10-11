#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QIODevice;
class QSlider;
class QSpinBox;

namespace pictura {

class ImageView;
class PictureView;

// Image > Adjustments > Match Color (#352): match the active layer's colours
// to a source document, one of its layers, or the loaded statistics, with a
// Lab mean/std transfer. Modal, like the other adjustment dialogs: the
// statistics are gathered at open and when the source changes, the sliders
// preview live, and OK commits exactly one "Match Color" state. Adobe's
// kernel is closed; the transfer is the approximation pinned in the
// add-match-color OpenSpec change.
class MatchColorDialog : public QDialog {
    Q_OBJECT

public:
    // One candidate source document, by tab title. The combo's "None" entry
    // (the target itself) is implicit; `sources` holds only real documents.
    struct Source {
        QString name;
        PictureView* view = nullptr;
    };

    MatchColorDialog(PictureView* view, ImageView* canvas, const QList<Source>& sources,
                     QWidget* parent = nullptr);

    // Read/write the six source statistics doubles (mean then std, Lab) as
    // the in-tree `.sta` payload: the ASCII magic `PCTMCST`, a version byte,
    // then six little-endian f64s. Adobe's own format is undocumented, so a
    // lossless Save->Load round trip is the contract. The device must be
    // open; public for the Qt Test.
    static QList<double> readStatistics(QIODevice* device);
    static bool writeStatistics(QIODevice* device, const QList<double>& statistics);

    void accept() override;
    void reject() override;

private:
    void buildUi();
    void rebuildLayerCombo();
    void recomputeStats();
    void applyPreview();
    void cancelPreview();
    void saveStatistics();
    void loadStatistics();

    PictureView* view_ = nullptr;
    ImageView* canvas_ = nullptr;
    QList<Source> sources_;

    QComboBox* sourceCombo_ = nullptr;
    QComboBox* layerCombo_ = nullptr;
    QList<QString> layerPaths_;
    QCheckBox* useSourceSelection_ = nullptr;
    QCheckBox* useTargetSelection_ = nullptr;
    QCheckBox* ignoreSelection_ = nullptr;
    QCheckBox* neutralize_ = nullptr;
    QSlider* luminanceSlider_ = nullptr;
    QSpinBox* luminanceSpin_ = nullptr;
    QSlider* intensitySlider_ = nullptr;
    QSpinBox* intensitySpin_ = nullptr;
    QSlider* fadeSlider_ = nullptr;
    QSpinBox* fadeSpin_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QList<double> targetStats_;
    QList<double> sourceStats_;
    bool previewing_ = false;
};

} // namespace pictura
