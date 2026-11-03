// Image > Apply Image (#609): see apply_image_dialog.h.

#include "apply_image_dialog.h"

#include "api/layers_api.h"
#include "blend_modes.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/channels.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_apply_image.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <optional>

namespace pictura {

namespace {

// The bridge's channel codes (`apply_image_read`).
constexpr int kComposite = 0;
constexpr int kTransparency = -1;
constexpr int kLayerMask = -2;
constexpr int kSelection = -3;
constexpr int kNoMask = -100;
constexpr int kAlphaBase = 100;

const QString kAdd = QStringLiteral("add");
const QString kSubtract = QStringLiteral("subtract");
const QString kDissolveKey = QStringLiteral("diss");
const QString kLayerSubtractKey = QStringLiteral("fsub");
const QString kMultiplyKey = QStringLiteral("mul ");

QString compositeName(const QString& mode)
{
    return mode == QLatin1String("grayscale") ? QStringLiteral("Gray") : mode.toUpper();
}

} // namespace

struct ApplyImageDialog::Snapshots {
    std::optional<::rust::Box<ApplyImageSnapshot>> source;
    std::optional<::rust::Box<ApplyImageSnapshot>> mask;
};

ApplyImageDialog::ApplyImageDialog(PictureView* view, ImageView* canvas,
                                   const QList<Source>& documents, QWidget* parent)
    : QDialog(parent), view_(view), canvas_(canvas), snapshots_(std::make_unique<Snapshots>())
{
    setWindowTitle(tr("Apply Image"));
    for (const Source& document : documents) {
        if (document.view == view_) {
            sources_.prepend(document);
        } else if (document.view && view_ && apply_image_same_size(*document.view, *view_)) {
            sources_.append(document);
        }
    }
    buildUi();
    readSource();
    readMask();
    applyPreview();
}

ApplyImageDialog::~ApplyImageDialog() = default;

ApplyImageDialog::Picker ApplyImageDialog::makePicker(const QString& prefix)
{
    Picker picker;
    picker.source = new QComboBox;
    picker.source->setObjectName(prefix + QStringLiteral("Source"));
    for (const Source& source : sources_) {
        picker.source->addItem(source.name);
    }
    picker.layer = new QComboBox;
    picker.layer->setObjectName(prefix + QStringLiteral("Layer"));
    picker.channel = new QComboBox;
    picker.channel->setObjectName(prefix + QStringLiteral("Channel"));
    return picker;
}

void ApplyImageDialog::buildUi()
{
    auto* source = new QGroupBox(tr("Source"));
    auto* sourceForm = new QFormLayout(source);
    source_ = makePicker(QStringLiteral("applyImage"));
    invert_ = new QCheckBox(tr("Invert"));
    invert_->setObjectName(QStringLiteral("applyImageInvert"));
    auto* channelRow = new QHBoxLayout;
    channelRow->addWidget(source_.channel, 1);
    channelRow->addWidget(invert_);
    sourceForm->addRow(tr("Source:"), source_.source);
    sourceForm->addRow(tr("Layer:"), source_.layer);
    sourceForm->addRow(tr("Channel:"), channelRow);

    auto* target = new QLabel(view_ ? QString(apply_image_target_label(*view_)) : QString());
    target->setObjectName(QStringLiteral("applyImageTarget"));

    auto* blendingBox = new QGroupBox(tr("Blending"));
    auto* blendingForm = new QFormLayout(blendingBox);
    blending_ = new QComboBox;
    blending_->setObjectName(QStringLiteral("applyImageBlending"));
    // ponytail: Dissolve needs a positional dither this kernel does not have.
    for (const BlendEntry& entry : blendModes()) {
        if (entry.key == kDissolveKey) {
            continue;
        }
        if (entry.key == kLayerSubtractKey) {
            blending_->addItem(tr("Add"), kAdd);
            blending_->addItem(tr("Subtract"), kSubtract);
            continue;
        }
        blending_->addItem(entry.name, entry.key);
    }
    blending_->setCurrentIndex(blending_->findData(kMultiplyKey));
    blendingForm->addRow(tr("Blending:"), blending_);
    opacity_ = new QSpinBox;
    opacity_->setObjectName(QStringLiteral("applyImageOpacity"));
    opacity_->setRange(0, 100);
    opacity_->setValue(100);
    opacity_->setSuffix(QStringLiteral(" %"));
    blendingForm->addRow(tr("Opacity:"), opacity_);
    scaleLabel_ = new QLabel(tr("Scale:"));
    scale_ = new QDoubleSpinBox;
    scale_->setObjectName(QStringLiteral("applyImageScale"));
    scale_->setDecimals(3);
    scale_->setRange(1.0, 2.0);
    scale_->setSingleStep(0.1);
    scale_->setValue(1.0);
    blendingForm->addRow(scaleLabel_, scale_);
    offsetLabel_ = new QLabel(tr("Offset:"));
    offset_ = new QSpinBox;
    offset_->setObjectName(QStringLiteral("applyImageOffset"));
    offset_->setRange(-255, 255);
    blendingForm->addRow(offsetLabel_, offset_);
    preserve_ = new QCheckBox(tr("Preserve Transparency"));
    preserve_->setObjectName(QStringLiteral("applyImagePreserve"));
    blendingForm->addRow(QString(), preserve_);
    useMask_ = new QCheckBox(tr("Mask…"));
    useMask_->setObjectName(QStringLiteral("applyImageUseMask"));
    blendingForm->addRow(QString(), useMask_);

    maskGroup_ = new QGroupBox(tr("Mask"));
    auto* maskForm = new QFormLayout(maskGroup_);
    mask_ = makePicker(QStringLiteral("applyImageMask"));
    maskInvert_ = new QCheckBox(tr("Invert"));
    maskInvert_->setObjectName(QStringLiteral("applyImageMaskInvert"));
    auto* maskChannelRow = new QHBoxLayout;
    maskChannelRow->addWidget(mask_.channel, 1);
    maskChannelRow->addWidget(maskInvert_);
    maskForm->addRow(tr("Image:"), mask_.source);
    maskForm->addRow(tr("Layer:"), mask_.layer);
    maskForm->addRow(tr("Channel:"), maskChannelRow);
    maskGroup_->setVisible(false);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         Qt::Vertical);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    preview_ = new QCheckBox(tr("Preview"));
    preview_->setObjectName(QStringLiteral("applyImagePreview"));
    preview_->setChecked(true);

    auto* left = new QVBoxLayout;
    left->addWidget(source);
    auto* targetRow = new QHBoxLayout;
    targetRow->addWidget(new QLabel(tr("Target:")));
    targetRow->addWidget(target, 1);
    left->addLayout(targetRow);
    left->addWidget(blendingBox);
    left->addWidget(maskGroup_);
    auto* right = new QVBoxLayout;
    right->addWidget(buttons);
    right->addWidget(preview_);
    right->addStretch(1);
    auto* outer = new QHBoxLayout(this);
    outer->addLayout(left, 1);
    outer->addLayout(right);

    rebuildLayers(source_, false);
    rebuildLayers(mask_, true);
    updateModeRows();

    const auto rePreview = [this]() { applyPreview(); };
    const auto sourceRead = [this]() {
        readSource();
        applyPreview();
    };
    const auto maskRead = [this]() {
        readMask();
        applyPreview();
    };
    connect(source_.source, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildLayers(source_, false);
        sourceRead();
    });
    connect(source_.layer, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildChannels(source_, false);
        sourceRead();
    });
    connect(source_.channel, &QComboBox::currentIndexChanged, this, sourceRead);
    connect(mask_.source, &QComboBox::currentIndexChanged, this, [this, maskRead]() {
        rebuildLayers(mask_, true);
        maskRead();
    });
    connect(mask_.layer, &QComboBox::currentIndexChanged, this, [this, maskRead]() {
        rebuildChannels(mask_, true);
        maskRead();
    });
    connect(mask_.channel, &QComboBox::currentIndexChanged, this, maskRead);
    connect(useMask_, &QCheckBox::toggled, this, [this, maskRead](bool on) {
        maskGroup_->setVisible(on);
        maskRead();
    });
    connect(blending_, &QComboBox::currentIndexChanged, this, [this]() {
        updateModeRows();
        applyPreview();
    });
    for (QCheckBox* box : {invert_, preserve_, maskInvert_, preview_}) {
        connect(box, &QCheckBox::toggled, this, rePreview);
    }
    connect(opacity_, &QSpinBox::valueChanged, this, rePreview);
    connect(offset_, &QSpinBox::valueChanged, this, rePreview);
    connect(scale_, &QDoubleSpinBox::valueChanged, this, rePreview);
}

void ApplyImageDialog::rebuildLayers(const Picker& picker, bool mask)
{
    {
        const QSignalBlocker block(picker.layer);
        picker.layer->clear();
        const int index = picker.source->currentIndex();
        PictureView* source = index >= 0 && index < sources_.size() ? sources_.at(index).view : nullptr;
        if (source) {
            picker.layer->addItem(tr("Merged"), QString());
            picker.layer->insertSeparator(1);
            for (const auto& row : layers_api::rows(*source)) {
                if (row.kind == QLatin1String("pixel") || row.kind == QLatin1String("background")
                    || row.kind == QLatin1String("type")) {
                    picker.layer->addItem(row.name, row.path);
                }
            }
        }
        picker.layer->setCurrentIndex(0);
    }
    rebuildChannels(picker, mask);
}

void ApplyImageDialog::rebuildChannels(const Picker& picker, bool mask)
{
    const QSignalBlocker block(picker.channel);
    picker.channel->clear();
    const int index = picker.source->currentIndex();
    PictureView* source = index >= 0 && index < sources_.size() ? sources_.at(index).view : nullptr;
    if (!source || !view_) {
        return;
    }
    const QString mode = QString(channels_mode(*source));
    const bool gray = mode == QLatin1String("grayscale");
    const bool sameMode = mode == QString(channels_mode(*view_));
    if (mask && source->has_selection()) {
        picker.channel->addItem(tr("Selection"), kSelection);
    }
    if (sameMode) {
        picker.channel->addItem(compositeName(mode), kComposite);
    }
    if (gray) {
        if (!sameMode) {
            picker.channel->addItem(tr("Gray"), 1);
        }
    } else {
        // ponytail: a CMYK or Lab source lists the RGB working planes.
        picker.channel->addItem(tr("Red"), 1);
        picker.channel->addItem(tr("Green"), 2);
        picker.channel->addItem(tr("Blue"), 3);
    }
    picker.channel->addItem(tr("Transparency"), kTransparency);
    const QString path = picker.layer->currentData().toString();
    if (!path.isEmpty()) {
        for (const auto& row : layers_api::rows(*source)) {
            if (row.path == path && row.hasMask) {
                picker.channel->addItem(tr("Layer Mask"), kLayerMask);
            }
        }
    }
    const int alphas = source->selection_channel_count();
    for (int i = 0; i < alphas; ++i) {
        picker.channel->addItem(tr("Alpha %1").arg(i + 1), kAlphaBase + i);
    }
    picker.channel->setCurrentIndex(0);
}

void ApplyImageDialog::readSource()
{
    // The active document may be the source: read its unpreviewed pixels.
    cancelPreview();
    const int index = source_.source->currentIndex();
    if (index < 0 || index >= sources_.size() || !sources_.at(index).view) {
        snapshots_->source.reset();
        return;
    }
    snapshots_->source = apply_image_read(*sources_.at(index).view,
                                          source_.layer->currentData().toString(),
                                          source_.channel->currentData().toInt());
}

void ApplyImageDialog::readMask()
{
    cancelPreview();
    const int index = mask_.source->currentIndex();
    if (!useMask_->isChecked() || index < 0 || index >= sources_.size()
        || !sources_.at(index).view) {
        snapshots_->mask = apply_image_read(*view_, QString(), kNoMask);
        return;
    }
    snapshots_->mask = apply_image_read(*sources_.at(index).view,
                                        mask_.layer->currentData().toString(),
                                        mask_.channel->currentData().toInt());
}

void ApplyImageDialog::updateModeRows()
{
    const QString mode = blending_->currentData().toString();
    const bool arithmetic = mode == kAdd || mode == kSubtract;
    scaleLabel_->setVisible(arithmetic);
    scale_->setVisible(arithmetic);
    offsetLabel_->setVisible(arithmetic);
    offset_->setVisible(arithmetic);
}

void ApplyImageDialog::applyPreview()
{
    if (!view_ || !snapshots_->source || !snapshots_->mask || !preview_->isChecked()) {
        cancelPreview();
        return;
    }
    // The visible section moves as the user pans or zooms, so read it live.
    const QRect visible =
        canvas_ ? canvas_->visibleDocumentRect().toAlignedRect() : QRect();
    if (image_apply_image_preview(*view_, **snapshots_->source, invert_->isChecked(),
                                  blending_->currentData().toString(), scale_->value(),
                                  offset_->value(), opacity_->value(), preserve_->isChecked(),
                                  **snapshots_->mask, maskInvert_->isChecked(), visible.x(),
                                  visible.y(), visible.width(), visible.height())) {
        previewing_ = true;
    } else {
        previewing_ = false;
    }
}

void ApplyImageDialog::cancelPreview()
{
    if (view_ && previewing_) {
        filter_preview_cancel(*view_);
    }
    previewing_ = false;
}

void ApplyImageDialog::accept()
{
    // The commit re-applies to the whole target from the pre-preview pixels.
    if (view_ && snapshots_->source && snapshots_->mask
        && image_apply_image_apply(*view_, **snapshots_->source, invert_->isChecked(),
                                   blending_->currentData().toString(), scale_->value(),
                                   offset_->value(), opacity_->value(), preserve_->isChecked(),
                                   **snapshots_->mask, maskInvert_->isChecked())) {
        previewing_ = false;
    } else {
        cancelPreview();
    }
    QDialog::accept();
}

void ApplyImageDialog::reject()
{
    cancelPreview();
    QDialog::reject();
}

} // namespace pictura

#include "moc_apply_image_dialog.cpp"
