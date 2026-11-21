// Image > Calculations (#610): see calculations_dialog.h.

#include "calculations_dialog.h"

#include "api/layers_api.h"
#include "blend_modes.h"

#include "api/filters_bridge.h"
#include "api/layers_bridge.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
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

#include <algorithm>
#include <optional>

namespace pictura {

namespace {

// The Apply Image bridge's channel codes (`apply_image_read`).
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

} // namespace

struct CalculationsDialog::Snapshots {
    std::optional<::rust::Box<ApplyImageSnapshot>> source1;
    std::optional<::rust::Box<ApplyImageSnapshot>> source2;
    std::optional<::rust::Box<ApplyImageSnapshot>> mask;
};

CalculationsDialog::CalculationsDialog(PictureView* view, const QList<Source>& documents,
                                       QWidget* parent)
    : QDialog(parent), view_(view), snapshots_(std::make_unique<Snapshots>())
{
    setWindowTitle(tr("Calculations"));
    for (const Source& document : documents) {
        if (document.view == view_) {
            sources_.prepend(document);
        } else if (document.view && view_ && apply_image_same_size(*document.view, *view_)) {
            sources_.append(document);
        }
    }
    buildUi();
    readSources();
    updatePreview();
}

CalculationsDialog::~CalculationsDialog() = default;

CalculationsDialog::Picker CalculationsDialog::makePicker(const QString& prefix)
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
    picker.invert = new QCheckBox(tr("Invert"));
    picker.invert->setObjectName(prefix + QStringLiteral("Invert"));
    return picker;
}

void CalculationsDialog::buildUi()
{
    const auto sourceBox = [this](const QString& title, Picker& picker, const QString& prefix) {
        auto* box = new QGroupBox(title);
        auto* form = new QFormLayout(box);
        picker = makePicker(prefix);
        auto* channelRow = new QHBoxLayout;
        channelRow->addWidget(picker.channel, 1);
        channelRow->addWidget(picker.invert);
        form->addRow(tr("Source:"), picker.source);
        form->addRow(tr("Layer:"), picker.layer);
        form->addRow(tr("Channel:"), channelRow);
        return box;
    };
    auto* first = sourceBox(tr("Source 1"), source1_, QStringLiteral("calculationsSource1"));
    auto* second = sourceBox(tr("Source 2"), source2_, QStringLiteral("calculationsSource2"));

    auto* blendingBox = new QGroupBox(tr("Blending"));
    auto* blendingForm = new QFormLayout(blendingBox);
    blending_ = new QComboBox;
    blending_->setObjectName(QStringLiteral("calculationsBlending"));
    // ponytail: Dissolve needs a positional dither this kernel does not have,
    // the same ceiling as Apply Image.
    for (const BlendEntry& entry : blendModes()) {
        if (entry.key == kDissolveKey) {
            continue;
        }
        if (entry.key == kLayerSubtractKey) {
            blending_->addItem(tr("Add"), kAdd);
            blending_->addItem(tr("Subtract"), kSubtract);
            continue;
        }
        blending_->addItem(entry.displayName(), entry.key);
    }
    blending_->setCurrentIndex(blending_->findData(kMultiplyKey));
    blendingForm->addRow(tr("Blending:"), blending_);
    opacity_ = new QSpinBox;
    opacity_->setObjectName(QStringLiteral("calculationsOpacity"));
    opacity_->setRange(0, 100);
    opacity_->setValue(100);
    opacity_->setSuffix(QStringLiteral(" %"));
    blendingForm->addRow(tr("Opacity:"), opacity_);
    scaleLabel_ = new QLabel(tr("Scale:"));
    scale_ = new QDoubleSpinBox;
    scale_->setObjectName(QStringLiteral("calculationsScale"));
    scale_->setDecimals(3);
    scale_->setRange(1.0, 2.0);
    scale_->setSingleStep(0.1);
    scale_->setValue(1.0);
    blendingForm->addRow(scaleLabel_, scale_);
    offsetLabel_ = new QLabel(tr("Offset:"));
    offset_ = new QSpinBox;
    offset_->setObjectName(QStringLiteral("calculationsOffset"));
    offset_->setRange(-255, 255);
    blendingForm->addRow(offsetLabel_, offset_);
    useMask_ = new QCheckBox(tr("Mask…"));
    useMask_->setObjectName(QStringLiteral("calculationsUseMask"));
    blendingForm->addRow(QString(), useMask_);

    maskGroup_ = new QGroupBox(tr("Mask"));
    auto* maskForm = new QFormLayout(maskGroup_);
    mask_ = makePicker(QStringLiteral("calculationsMask"));
    auto* maskChannelRow = new QHBoxLayout;
    maskChannelRow->addWidget(mask_.channel, 1);
    maskChannelRow->addWidget(mask_.invert);
    maskForm->addRow(tr("Image:"), mask_.source);
    maskForm->addRow(tr("Layer:"), mask_.layer);
    maskForm->addRow(tr("Channel:"), maskChannelRow);
    maskGroup_->setVisible(false);

    result_ = new QComboBox;
    result_->setObjectName(QStringLiteral("calculationsResult"));
    result_->addItem(tr("New Document"), QStringLiteral("document"));
    result_->addItem(tr("New Channel"), QStringLiteral("channel"));
    result_->addItem(tr("Selection"), QStringLiteral("selection"));

    previewImage_ = new QLabel;
    previewImage_->setObjectName(QStringLiteral("calculationsPreviewImage"));
    previewImage_->setAlignment(Qt::AlignCenter);
    previewImage_->setFixedSize(160, 160);
    previewImage_->setFrameShape(QFrame::StyledPanel);
    preview_ = new QCheckBox(tr("Preview"));
    preview_->setObjectName(QStringLiteral("calculationsPreview"));
    preview_->setChecked(true);
    warning_ = new QLabel;
    warning_->setObjectName(QStringLiteral("calculationsWarning"));
    warning_->setWordWrap(true);
    warning_->setVisible(false);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         Qt::Vertical);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { accept(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* left = new QVBoxLayout;
    left->addWidget(first);
    left->addWidget(second);
    left->addWidget(blendingBox);
    left->addWidget(maskGroup_);

    auto* right = new QVBoxLayout;
    auto* resultRow = new QHBoxLayout;
    resultRow->addWidget(new QLabel(tr("Result:")));
    resultRow->addWidget(result_, 1);
    right->addLayout(resultRow);
    right->addWidget(previewImage_, 0, Qt::AlignHCenter);
    right->addWidget(preview_);
    right->addWidget(warning_);
    right->addStretch(1);
    right->addWidget(buttons);

    auto* outer = new QHBoxLayout(this);
    outer->addLayout(left, 1);
    outer->addLayout(right);

    rebuildLayers(source1_, false);
    rebuildLayers(source2_, false);
    rebuildLayers(mask_, true);
    updateModeRows();

    const auto rePreview = [this]() { updatePreview(); };
    const auto sourceRead = [this]() {
        readSources();
        updatePreview();
    };
    connect(source1_.source, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildLayers(source1_, false);
        sourceRead();
    });
    connect(source1_.layer, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildChannels(source1_, false);
        sourceRead();
    });
    connect(source1_.channel, &QComboBox::currentIndexChanged, this, sourceRead);
    connect(source1_.invert, &QCheckBox::toggled, this, rePreview);
    connect(source2_.source, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildLayers(source2_, false);
        sourceRead();
    });
    connect(source2_.layer, &QComboBox::currentIndexChanged, this, [this, sourceRead]() {
        rebuildChannels(source2_, false);
        sourceRead();
    });
    connect(source2_.channel, &QComboBox::currentIndexChanged, this, sourceRead);
    connect(source2_.invert, &QCheckBox::toggled, this, rePreview);

    const auto maskRead = [this]() {
        readSources();
        updatePreview();
    };
    connect(mask_.source, &QComboBox::currentIndexChanged, this, [this, maskRead]() {
        rebuildLayers(mask_, true);
        maskRead();
    });
    connect(mask_.layer, &QComboBox::currentIndexChanged, this, [this, maskRead]() {
        rebuildChannels(mask_, true);
        maskRead();
    });
    connect(mask_.channel, &QComboBox::currentIndexChanged, this, maskRead);
    connect(mask_.invert, &QCheckBox::toggled, this, rePreview);
    connect(useMask_, &QCheckBox::toggled, this, [this, maskRead](bool on) {
        maskGroup_->setVisible(on);
        maskRead();
    });

    connect(blending_, &QComboBox::currentIndexChanged, this, [this, rePreview]() {
        updateModeRows();
        rePreview();
    });
    connect(opacity_, &QSpinBox::valueChanged, this, rePreview);
    connect(scale_, &QDoubleSpinBox::valueChanged, this, rePreview);
    connect(offset_, &QSpinBox::valueChanged, this, rePreview);
    connect(preview_, &QCheckBox::toggled, this, rePreview);
    connect(result_, &QComboBox::currentIndexChanged, this, [this]() {
        warning_->setVisible(false);
    });
}

void CalculationsDialog::rebuildLayers(const Picker& picker, bool mask)
{
    {
        const QSignalBlocker block(picker.layer);
        picker.layer->clear();
        const int index = picker.source->currentIndex();
        PictureView* source =
            index >= 0 && index < sources_.size() ? sources_.at(index).view : nullptr;
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

void CalculationsDialog::rebuildChannels(const Picker& picker, bool mask)
{
    const QSignalBlocker block(picker.channel);
    picker.channel->clear();
    const int index = picker.source->currentIndex();
    PictureView* source =
        index >= 0 && index < sources_.size() ? sources_.at(index).view : nullptr;
    if (!source || !view_) {
        return;
    }
    const QString mode = QString(channels_mode(*source));
    const bool gray = mode == QLatin1String("grayscale");
    picker.channel->addItem(tr("Gray"), kComposite);
    if (!gray) {
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
    const QStringList alphas = channels_alpha_names(*source);
    for (int i = 0; i < alphas.size(); ++i) {
        picker.channel->addItem(alphas.at(i), kAlphaBase + i);
    }
    if (mask && source->has_selection()) {
        picker.channel->addItem(tr("Selection"), kSelection);
    }
    picker.channel->setCurrentIndex(0);
}

void CalculationsDialog::readSources()
{
    const auto read = [this](const Picker& picker, bool mask) {
        const int index = picker.source->currentIndex();
        if (index < 0 || index >= sources_.size() || !sources_.at(index).view) {
            return std::optional<::rust::Box<ApplyImageSnapshot>>{};
        }
        if (mask && !useMask_->isChecked()) {
            return std::optional<::rust::Box<ApplyImageSnapshot>>{
                apply_image_read(*view_, QString(), kNoMask)};
        }
        return std::optional<::rust::Box<ApplyImageSnapshot>>{apply_image_read(
            *sources_.at(index).view, picker.layer->currentData().toString(),
            picker.channel->currentData().toInt())};
    };
    snapshots_->source1 = read(source1_, false);
    snapshots_->source2 = read(source2_, false);
    snapshots_->mask = read(mask_, true);
}

void CalculationsDialog::updateModeRows()
{
    const QString mode = blending_->currentData().toString();
    const bool arithmetic = mode == kAdd || mode == kSubtract;
    scaleLabel_->setVisible(arithmetic);
    scale_->setVisible(arithmetic);
    offsetLabel_->setVisible(arithmetic);
    offset_->setVisible(arithmetic);
}

void CalculationsDialog::updatePreview()
{
    previewImage_->clear();
    if (!preview_->isChecked() || !snapshots_->source1 || !snapshots_->source2
        || !snapshots_->mask) {
        return;
    }
    int width = 0;
    int height = 0;
    const auto bytes = calculations_preview(
        **snapshots_->source1, source1_.invert->isChecked(), **snapshots_->source2,
        source2_.invert->isChecked(), blending_->currentData().toString(), scale_->value(),
        offset_->value(), opacity_->value(), **snapshots_->mask, mask_.invert->isChecked(),
        width, height);
    if (width <= 0 || height <= 0 || int(bytes.size()) != width * height) {
        return;
    }
    QImage raw(width, height, QImage::Format_Grayscale8);
    for (int y = 0; y < height; ++y) {
        std::copy_n(bytes.data() + std::size_t(y) * std::size_t(width), width, raw.scanLine(y));
    }
    previewImage_->setPixmap(QPixmap::fromImage(raw.scaled(
        previewImage_->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation)));
}

CalculationsDialog::Result CalculationsDialog::result() const
{
    const QString target = result_->currentData().toString();
    if (target == QLatin1String("channel")) {
        return Result::NewChannel;
    }
    if (target == QLatin1String("selection")) {
        return Result::Selection;
    }
    return Result::NewDocument;
}

bool CalculationsDialog::apply(PictureView* target) const
{
    if (!target || !snapshots_->source1 || !snapshots_->source2 || !snapshots_->mask) {
        return false;
    }
    const auto& source1 = **snapshots_->source1;
    const auto& source2 = **snapshots_->source2;
    const auto& mask = **snapshots_->mask;
    const QString mode = blending_->currentData().toString();
    const bool invert1 = source1_.invert->isChecked();
    const bool invert2 = source2_.invert->isChecked();
    const bool maskInvert = mask_.invert->isChecked();
    if (result() == Result::NewDocument) {
        return calculations_document(*target, source1, invert1, source2, invert2, mode,
                                      scale_->value(), offset_->value(), opacity_->value(),
                                      mask, maskInvert);
    }
    const QString targetName =
        result() == Result::NewChannel ? QStringLiteral("channel") : QStringLiteral("selection");
    return calculations_apply(*target, source1, invert1, source2, invert2, mode, scale_->value(),
                              offset_->value(), opacity_->value(), mask, maskInvert, targetName);
}

void CalculationsDialog::accept()
{
    if (result() == Result::NewChannel && view_ && !channels_can_add(*view_)) {
        warning_->setText(tr("The document is at its 56-channel limit."));
        warning_->setVisible(true);
        return;
    }
    QDialog::accept();
}

} // namespace pictura

#include "moc_calculations_dialog.cpp"
