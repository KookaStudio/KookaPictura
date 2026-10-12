#include "gradient_map_dialog.h"

#include "dialogs.h"

#include "pictura_app/src/cxxqt_object/image_adjust/gradient_map.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

::rust::Slice<const std::uint8_t> slice(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), std::size_t(bytes.size())};
}

constexpr QSize kSampleSize(250, 20);

} // namespace

GradientMapDialog::GradientMapDialog(PictureView* view, const QByteArray& block,
                                     const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Gradient Map"), parent)
{
    spec_.name = QString(gradient_preset_name(0));
    spec_.stops = parseGradientStops(QString(gradient_map_stops(slice(block))));
    // The opening block runs foreground to background
    // (`image_adjustment_default`), so its ends are the colours the presets
    // that name them use.
    foreground_ = spec_.stops.isEmpty() ? QColor(Qt::black) : spec_.stops.first().color;
    background_ = spec_.stops.isEmpty() ? QColor(Qt::white) : spec_.stops.last().color;

    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;

    auto* mapping = new QGroupBox(QStringLiteral("Gradient Used for Grayscale Mapping"), this);
    auto* mappingRow = new QHBoxLayout(mapping);
    mappingRow->setSpacing(0);
    sample_ = new QToolButton(mapping);
    sample_->setObjectName(QStringLiteral("gradientMapSample"));
    sample_->setIconSize(kSampleSize);
    sample_->setToolTip(QStringLiteral("Click to edit the gradient"));
    presets_ = new QToolButton(mapping);
    presets_->setObjectName(QStringLiteral("gradientMapPresets"));
    presets_->setArrowType(Qt::DownArrow);
    presets_->setPopupMode(QToolButton::InstantPopup);
    presets_->setToolTip(QStringLiteral("Click to open Gradient picker"));
    presets_->setFixedHeight(kSampleSize.height() + 8);
    auto* menu = new QMenu(presets_);
    for (int i = 0; i < gradient_preset_count(); ++i) {
        QPixmap pixmap(32, 16);
        {
            QPainter painter(&pixmap);
            paintGradient(painter, pixmap.rect(), gradientPreset(i, foreground_, background_));
        }
        const QString name(gradient_preset_name(i));
        connect(menu->addAction(QIcon(pixmap), name), &QAction::triggered, this, [this, i, name]() {
            GradientSpec preset;
            preset.name = name;
            preset.stops = gradientPreset(i, foreground_, background_);
            preset.noise = spec_.noise;
            setSpec(preset);
        });
    }
    presets_->setMenu(menu);
    mappingRow->addWidget(sample_, 1);
    mappingRow->addWidget(presets_);
    left->addWidget(mapping);

    auto* options = new QGroupBox(QStringLiteral("Gradient Options"), this);
    auto* optionsColumn = new QVBoxLayout(options);
    dither_ = new QCheckBox(QStringLiteral("Dither"), options);
    dither_->setObjectName(QStringLiteral("dither"));
    reverse_ = new QCheckBox(QStringLiteral("Reverse"), options);
    reverse_->setObjectName(QStringLiteral("reverse"));
    reverse_->setChecked(gradient_map_reverse(slice(block)));
    // ponytail: the map is not dithered, so Dither stays off-limits; the
    // block's flag is carried through unchanged.
    dither_->setChecked(gradient_map_dither(slice(block)));
    dither_->setEnabled(false);
    dither_->setToolTip(QStringLiteral("Dithering is not supported yet"));
    optionsColumn->addWidget(dither_);
    optionsColumn->addWidget(reverse_);
    left->addWidget(options);
    left->addStretch(1);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn();
    buttons->addSpacing(10);
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    paintSample();
    connect(sample_, &QToolButton::clicked, this, [this]() { openEditor(); });
    connect(reverse_, &QCheckBox::toggled, this, &GradientMapDialog::rebuild);
}

void GradientMapDialog::setSpec(const GradientSpec& spec)
{
    if (spec.type == GradientSpec::Solid && spec.stops.size() < 2) {
        return;
    }
    spec_ = spec;
    paintSample();
    rebuild();
}

void GradientMapDialog::openEditor(const std::function<void(GradientEditorDialog&)>& editor)
{
    const GradientSpec before = spec_;
    GradientEditorDialog dialog(spec_, foreground_, background_, this);
    connect(&dialog, &GradientEditorDialog::gradientChanged, this,
            [this, &dialog]() { setSpec(dialog.spec()); });
    if (editor) {
        editor(dialog);
    }
    if (runDialog(dialog, this) == QDialog::Accepted) {
        setSpec(dialog.spec());
    } else {
        setSpec(before);
    }
}

void GradientMapDialog::rebuild()
{
    // Smoothness and Noise are baked into the stops, so the block gets them
    // at 0 %.
    const ::rust::Vec<std::uint8_t> next = gradient_map_block(
        formatGradientStops(resolvedStops(spec_)), 0, reverse_->isChecked(), dither_->isChecked());
    setBlock(QByteArray(reinterpret_cast<const char*>(next.data()), qsizetype(next.size())));
}

void GradientMapDialog::paintSample()
{
    QPixmap pixmap(kSampleSize);
    {
        QPainter painter(&pixmap);
        paintGradient(painter, pixmap.rect(), resolvedStops(spec_));
    }
    sample_->setIcon(QIcon(pixmap));
}

QWidget* GradientMapDialog::controlForTest(const QString& key) const
{
    if (key == QLatin1String("sample")) {
        return sample_;
    }
    if (key == QLatin1String("presets")) {
        return presets_;
    }
    return findChild<QWidget*>(key);
}

} // namespace pictura
