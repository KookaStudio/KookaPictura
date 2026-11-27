#include "auto_correction_options.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"
#include "preferences.h"

#include <QtCore/QCoreApplication>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

AutoCorrectionSettings autoCorrectionSettings(const Preferences& p)
{
    return {p.autoCorrectionAlgorithm, p.autoCorrectionSnap, p.autoCorrectionShadowClip,
            p.autoCorrectionHighlightClip, quint32(p.autoCorrectionShadow),
            quint32(p.autoCorrectionMidtone), quint32(p.autoCorrectionHighlight)};
}

bool saveAutoCorrectionSettings(const AutoCorrectionSettings& settings)
{
    const auto before = loadPreferences();
    auto after = before;
    after.autoCorrectionSaved = true;
    after.autoCorrectionAlgorithm = settings.algorithm;
    after.autoCorrectionSnap = settings.snap;
    after.autoCorrectionShadowClip = settings.shadow_clip;
    after.autoCorrectionHighlightClip = settings.highlight_clip;
    after.autoCorrectionShadow = int(settings.shadow);
    after.autoCorrectionMidtone = int(settings.midtone);
    after.autoCorrectionHighlight = int(settings.highlight);
    return savePreferences(before, after);
}

AutoCorrectionOptionsDialog::AutoCorrectionOptionsDialog(
    const AutoCorrectionSettings& initial,
    std::function<void(const AutoCorrectionSettings&)> preview, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("autoCorrectionOptions"));
    setWindowTitle(QCoreApplication::translate("PicturaUi", "Auto Color Correction Options"));
    auto* layout = new QVBoxLayout(this);
    auto* group = new QGroupBox(QCoreApplication::translate("PicturaUi", "Algorithms"), this);
    auto* algorithmsLayout = new QVBoxLayout(group);
    algorithms_ = new QButtonGroup(this);
    const std::array algorithmLabels{
        QCoreApplication::translate("PicturaUi", "Enhance Monochromatic Contrast"),
        QCoreApplication::translate("PicturaUi", "Enhance Per Channel Contrast"),
        QCoreApplication::translate("PicturaUi", "Find Dark && Light Colors"),
        QCoreApplication::translate("PicturaUi", "Enhance Brightness and Contrast")};
    for (int index = 0; index < int(algorithmLabels.size()); ++index) {
        auto* radio = new QRadioButton(algorithmLabels[index], group);
        radio->setObjectName(QStringLiteral("autoCorrectionAlgorithm%1").arg(index));
        algorithms_->addButton(radio, index);
        algorithmsLayout->addWidget(radio);
        radio->setChecked(index == initial.algorithm);
    }
    snap_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Snap Neutral Midtones"), group);
    snap_->setObjectName(QStringLiteral("autoCorrectionSnap"));
    snap_->setChecked(initial.snap);
    algorithmsLayout->addWidget(snap_);
    layout->addWidget(group);

    auto* targets = new QGroupBox(QCoreApplication::translate("PicturaUi", "Target Colors and Clipping"), this);
    auto* form = new QFormLayout(targets);
    const auto makeClip = [&](const QString& name, double value) {
        auto* clip = new QDoubleSpinBox(targets);
        clip->setObjectName(name);
        clip->setDecimals(2);
        clip->setRange(0.0, 49.99);
        clip->setSingleStep(0.1);
        clip->setSuffix(QStringLiteral("%"));
        clip->setKeyboardTracking(false);
        clip->setValue(value);
        return clip;
    };
    shadowClip_ = makeClip(QStringLiteral("autoCorrectionShadowClip"), initial.shadow_clip);
    highlightClip_ = makeClip(QStringLiteral("autoCorrectionHighlightClip"), initial.highlight_clip);
    const std::array labels{
        QCoreApplication::translate("PicturaUi", "Shadows"),
        QCoreApplication::translate("PicturaUi", "Midtones"),
        QCoreApplication::translate("PicturaUi", "Highlights")};
    const std::array colors{initial.shadow, initial.midtone, initial.highlight};
    const auto changed = [this, preview = std::move(preview)]() { preview(settings()); };
    for (std::size_t index = 0; index < targets_.size(); ++index) {
        const QString label = labels[index];
        auto* swatch = new ColorSwatchWidget(targets);
        swatch->setObjectName(QStringLiteral("autoCorrectionTarget%1").arg(int(index)));
        swatch->setAccessibleName(label);
        swatch->setColor(QColor::fromRgb(colors[index]));
        targets_[index] = swatch;
        form->addRow(label, swatch);
        connect(swatch, &QPushButton::clicked, this, [this, swatch, label, changed] {
            const QColor color = ColorPickerDialog::getColor(swatch->color(), this, label);
            if (color.isValid()) {
                swatch->setColor(color);
                changed();
            }
        });
    }
    form->addRow(QCoreApplication::translate("PicturaUi", "Shadow Clip"), shadowClip_);
    form->addRow(QCoreApplication::translate("PicturaUi", "Highlight Clip"), highlightClip_);
    layout->addWidget(targets);
    const auto updateAvailability = [this, targets] {
        const int algorithm = algorithms_->checkedId();
        targets->setEnabled(algorithm != 3);
        snap_->setEnabled(algorithm == 2);
    };
    connect(algorithms_, &QButtonGroup::idClicked, this, [updateAvailability](int) { updateAvailability(); });
    updateAvailability();
    save_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Save as Defaults"), this);
    save_->setObjectName(QStringLiteral("autoCorrectionSaveDefaults"));
    layout->addWidget(save_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(algorithms_, &QButtonGroup::idClicked, this, changed);
    connect(snap_, &QCheckBox::toggled, this, changed);
    connect(shadowClip_, &QDoubleSpinBox::valueChanged, this, changed);
    connect(highlightClip_, &QDoubleSpinBox::valueChanged, this, changed);
}

AutoCorrectionSettings AutoCorrectionOptionsDialog::settings() const
{
    const auto rgb = [](const ColorSwatchWidget* swatch) { return swatch->color().rgb() & 0xffffffu; };
    return {algorithms_->checkedId(), snap_->isChecked(), shadowClip_->value(),
            highlightClip_->value(), rgb(targets_[0]), rgb(targets_[1]), rgb(targets_[2])};
}

bool AutoCorrectionOptionsDialog::saveAsDefaults() const
{
    return save_->isChecked();
}

} // namespace pictura
