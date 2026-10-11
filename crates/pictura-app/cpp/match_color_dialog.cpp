// Image > Adjustments > Match Color (#352): the Source / Layer combos pick
// where the colour statistics come from, the three selection checkboxes and
// the Image Options controls shape the transfer, and OK commits one
// "Match Color" state while Cancel restores the layer exactly.

#include "match_color_dialog.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_match_color.cxxqt.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <QtCore/QBuffer>
#include <QtCore/QDataStream>
#include <QtCore/QFile>
#include <QtCore/QSaveFile>
#include <QtCore/QSignalBlocker>

namespace pictura {

namespace {

// The combo's synthetic entries, ahead of the real source documents.
constexpr int kNoneSource = 0;
constexpr int kLoadedSource = 1;

constexpr int kLuminanceMin = 1;
constexpr int kLuminanceMax = 200;
constexpr int kFadeMax = 100;

// A slider plus its numeric field, moving together.
void bindSlider(QSlider* slider, QSpinBox* spin)
{
    QObject::connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
    QObject::connect(spin, &QSpinBox::valueChanged, slider, &QSlider::setValue);
}

QList<double> statsList(const ::rust::Vec<double>& values)
{
    QList<double> out;
    for (double value : values) {
        out.append(value);
    }
    return out;
}

} // namespace

MatchColorDialog::MatchColorDialog(PictureView* view, ImageView* canvas,
                                   const QList<Source>& sources, QWidget* parent)
    : QDialog(parent), view_(view), canvas_(canvas), sources_(sources)
{
    setObjectName(QStringLiteral("matchColorDialog"));
    setWindowTitle(tr("Match Color"));
    buildUi();
    recomputeStats();
    if (preview_->isChecked()) {
        applyPreview();
    }
}

QList<double> MatchColorDialog::readStatistics(QIODevice* device)
{
    QList<double> out;
    QDataStream stream(device);
    stream.setByteOrder(QDataStream::LittleEndian);
    char magic[7] = {};
    if (stream.readRawData(magic, 7) != 7 || qstrncmp(magic, "PCTMCST", 7) != 0) {
        return out;
    }
    quint8 version = 0;
    stream >> version;
    if (version != 1) {
        return out;
    }
    for (int i = 0; i < 6; ++i) {
        double value = 0.0;
        stream >> value;
        out.append(value);
    }
    return stream.status() == QDataStream::Ok ? out : QList<double>();
}

bool MatchColorDialog::writeStatistics(QIODevice* device, const QList<double>& statistics)
{
    if (statistics.size() != 6) {
        return false;
    }
    QDataStream stream(device);
    stream.setByteOrder(QDataStream::LittleEndian);
    if (stream.writeRawData("PCTMCST", 7) != 7) {
        return false;
    }
    stream << quint8(1);
    for (double value : statistics) {
        stream << value;
    }
    return stream.status() == QDataStream::Ok;
}

void MatchColorDialog::buildUi()
{
    auto* statistics = new QGroupBox(tr("Image Statistics"));
    auto* statisticsForm = new QFormLayout(statistics);
    sourceCombo_ = new QComboBox;
    sourceCombo_->setObjectName(QStringLiteral("matchColorSource"));
    sourceCombo_->addItem(tr("None"));
    sourceCombo_->addItem(tr("Loaded Statistics"));
    for (const Source& source : sources_) {
        sourceCombo_->addItem(source.name);
    }
    statisticsForm->addRow(tr("Source:"), sourceCombo_);
    layerCombo_ = new QComboBox;
    layerCombo_->setObjectName(QStringLiteral("matchColorLayer"));
    statisticsForm->addRow(tr("Layer:"), layerCombo_);
    useSourceSelection_ = new QCheckBox(tr("Use Selection In Source To Calculate Colors"));
    useSourceSelection_->setObjectName(QStringLiteral("matchColorUseSourceSelection"));
    statisticsForm->addRow(QString(), useSourceSelection_);
    useTargetSelection_ = new QCheckBox(tr("Use Selection In Target To Calculate Adjustment"));
    useTargetSelection_->setObjectName(QStringLiteral("matchColorUseTargetSelection"));
    statisticsForm->addRow(QString(), useTargetSelection_);
    auto* statisticsButtons = new QHBoxLayout;
    auto* save = new QPushButton(tr("Save Statistics…"));
    auto* load = new QPushButton(tr("Load Statistics…"));
    statisticsButtons->addWidget(save);
    statisticsButtons->addWidget(load);
    statisticsButtons->addStretch(1);
    statisticsForm->addRow(QString(), statisticsButtons);

    auto* destination = new QGroupBox(tr("Destination Image"));
    auto* destinationForm = new QFormLayout(destination);
    ignoreSelection_ = new QCheckBox(tr("Ignore Selection When Applying Adjustment"));
    ignoreSelection_->setObjectName(QStringLiteral("matchColorIgnoreSelection"));
    destinationForm->addRow(QString(), ignoreSelection_);

    auto* options = new QGroupBox(tr("Image Options"));
    auto* optionsForm = new QFormLayout(options);
    neutralize_ = new QCheckBox(tr("Neutralize"));
    neutralize_->setObjectName(QStringLiteral("matchColorNeutralize"));
    optionsForm->addRow(QString(), neutralize_);
    luminanceSlider_ = new QSlider(Qt::Horizontal);
    luminanceSpin_ = new QSpinBox;
    luminanceSpin_->setObjectName(QStringLiteral("matchColorLuminance"));
    luminanceSpin_->setRange(kLuminanceMin, kLuminanceMax);
    luminanceSpin_->setValue(100);
    luminanceSlider_->setRange(kLuminanceMin, kLuminanceMax);
    luminanceSlider_->setValue(100);
    bindSlider(luminanceSlider_, luminanceSpin_);
    optionsForm->addRow(tr("Luminance:"), luminanceSpin_);
    intensitySlider_ = new QSlider(Qt::Horizontal);
    intensitySpin_ = new QSpinBox;
    intensitySpin_->setObjectName(QStringLiteral("matchColorColorIntensity"));
    intensitySpin_->setRange(kLuminanceMin, kLuminanceMax);
    intensitySpin_->setValue(100);
    intensitySlider_->setRange(kLuminanceMin, kLuminanceMax);
    intensitySlider_->setValue(100);
    bindSlider(intensitySlider_, intensitySpin_);
    optionsForm->addRow(tr("Color Intensity:"), intensitySpin_);
    fadeSlider_ = new QSlider(Qt::Horizontal);
    fadeSpin_ = new QSpinBox;
    fadeSpin_->setObjectName(QStringLiteral("matchColorFade"));
    fadeSpin_->setRange(0, kFadeMax);
    fadeSpin_->setValue(0);
    fadeSlider_->setRange(0, kFadeMax);
    fadeSlider_->setValue(0);
    bindSlider(fadeSlider_, fadeSpin_);
    optionsForm->addRow(tr("Fade:"), fadeSpin_);

    preview_ = new QCheckBox(tr("Preview"));
    preview_->setObjectName(QStringLiteral("matchColorPreview"));
    preview_->setChecked(true);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* outer = new QVBoxLayout(this);
    outer->addWidget(statistics);
    outer->addWidget(destination);
    outer->addWidget(options);
    auto* bottom = new QHBoxLayout;
    bottom->addWidget(preview_);
    bottom->addStretch(1);
    bottom->addWidget(buttons);
    outer->addLayout(bottom);

    // "None" and "Loaded Statistics" have no layers to list; a real source
    // lists its pixel layers plus "Merged".
    const auto sourceChanged = [this]() {
        rebuildLayerCombo();
        recomputeStats();
        if (preview_->isChecked()) {
            applyPreview();
        }
    };
    connect(sourceCombo_, &QComboBox::currentIndexChanged, this, sourceChanged);
    connect(layerCombo_, &QComboBox::currentIndexChanged, this, [this]() {
        recomputeStats();
        if (preview_->isChecked()) {
            applyPreview();
        }
    });
    for (QCheckBox* box : {useSourceSelection_, useTargetSelection_}) {
        connect(box, &QCheckBox::toggled, this, [this]() {
            recomputeStats();
            if (preview_->isChecked()) {
                applyPreview();
            }
        });
    }
    const auto rePreview = [this]() {
        if (preview_->isChecked()) {
            applyPreview();
        } else {
            cancelPreview();
        }
    };
    for (QCheckBox* box : {ignoreSelection_, neutralize_, preview_}) {
        connect(box, &QCheckBox::toggled, this, rePreview);
    }
    for (QSpinBox* spin : {luminanceSpin_, intensitySpin_, fadeSpin_}) {
        connect(spin, &QSpinBox::valueChanged, this, rePreview);
    }
    connect(save, &QPushButton::clicked, this, [this]() { saveStatistics(); });
    connect(load, &QPushButton::clicked, this, [this]() { loadStatistics(); });

    rebuildLayerCombo();
}

void MatchColorDialog::rebuildLayerCombo()
{
    const QSignalBlocker block(layerCombo_);
    layerCombo_->clear();
    layerPaths_.clear();
    layerCombo_->setEnabled(sourceCombo_->currentIndex() >= kLoadedSource + 1);
    if (!layerCombo_->isEnabled()) {
        return;
    }
    PictureView* source = sources_.at(sourceCombo_->currentIndex() - kLoadedSource - 1).view;
    if (!source) {
        return;
    }
    layerCombo_->addItem(tr("Merged"));
    layerPaths_.append(QString());
    const int count = source->layer_row_count();
    for (int i = 0; i < count; ++i) {
        const QString kind = source->layer_row_kind(i);
        if (kind != QLatin1String("pixel") && kind != QLatin1String("background")
            && kind != QLatin1String("type")) {
            continue;
        }
        layerCombo_->addItem(source->layer_row_name(i));
        layerPaths_.append(source->layer_row_path(i));
    }
    layerCombo_->setCurrentIndex(0);
}

void MatchColorDialog::recomputeStats()
{
    if (!view_) {
        targetStats_.clear();
        sourceStats_.clear();
        return;
    }
    targetStats_ = statsList(
        match_color_target_stats(*view_, useTargetSelection_->isChecked()));
    const int index = sourceCombo_->currentIndex();
    if (index <= kNoneSource || index == kLoadedSource) {
        // "None" matches the target to itself; "Loaded Statistics" keeps
        // whatever the file put in sourceStats_.
        if (index == kNoneSource) {
            sourceStats_ = targetStats_;
        }
        return;
    }
    PictureView* source = sources_.at(index - kLoadedSource - 1).view;
    if (!source) {
        sourceStats_ = targetStats_;
        return;
    }
    const QString path = layerCombo_->currentIndex() > 0
            && layerCombo_->currentIndex() < layerPaths_.size()
        ? layerPaths_.at(layerCombo_->currentIndex())
        : QString();
    sourceStats_ = statsList(match_color_source_stats(
        *source, path, useSourceSelection_->isChecked()));
    // A source with nothing to read (a missing layer) degrades to a
    // self-match rather than a refused dialog.
    if (sourceStats_.isEmpty()) {
        sourceStats_ = targetStats_;
    }
}

void MatchColorDialog::applyPreview()
{
    if (!view_ || targetStats_.isEmpty() || sourceStats_.isEmpty()) {
        cancelPreview();
        return;
    }
    // The visible section moves as the user pans or zooms, so read it live.
    const QRect visible =
        canvas_ ? canvas_->visibleDocumentRect().toAlignedRect() : QRect();
    if (image_match_color_preview(*view_, targetStats_, sourceStats_,
                                  ignoreSelection_->isChecked(), neutralize_->isChecked(),
                                  luminanceSpin_->value(), intensitySpin_->value(),
                                  fadeSpin_->value(), visible.x(), visible.y(),
                                  visible.width(), visible.height())) {
        previewing_ = true;
    } else {
        cancelPreview();
    }
}

void MatchColorDialog::cancelPreview()
{
    if (view_ && previewing_) {
        filter_preview_cancel(*view_);
    }
    previewing_ = false;
}

void MatchColorDialog::saveStatistics()
{
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Save Statistics"), QString(), tr("Statistics (*.sta)"));
    if (path.isEmpty()) {
        return;
    }
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) || !writeStatistics(&file, sourceStats_)) {
        file.cancelWriting();
        return;
    }
    file.commit();
}

void MatchColorDialog::loadStatistics()
{
    const QString path = QFileDialog::getOpenFileName(
        this, tr("Load Statistics"), QString(), tr("Statistics (*.sta)"));
    if (path.isEmpty()) {
        return;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return;
    }
    const QList<double> loaded = readStatistics(&file);
    if (loaded.size() != 6) {
        return;
    }
    sourceStats_ = loaded;
    sourceCombo_->setCurrentIndex(kLoadedSource);
    if (preview_->isChecked()) {
        applyPreview();
    }
}

void MatchColorDialog::accept()
{
    // The commit re-applies to the whole layer from the pre-preview pixels.
    if (view_ && !targetStats_.isEmpty() && !sourceStats_.isEmpty()
        && image_match_color_apply(*view_, targetStats_, sourceStats_,
                                   ignoreSelection_->isChecked(), neutralize_->isChecked(),
                                   luminanceSpin_->value(), intensitySpin_->value(),
                                   fadeSpin_->value())) {
        previewing_ = false;
    } else {
        cancelPreview();
    }
    QDialog::accept();
}

void MatchColorDialog::reject()
{
    cancelPreview();
    QDialog::reject();
}

} // namespace pictura
