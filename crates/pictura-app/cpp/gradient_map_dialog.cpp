#include "gradient_map_dialog.h"
#include <QtCore/QCoreApplication>

#include "dialogs.h"
#include "preset_store.h"

#include "api/paint_bridge.h"
#include "api/filters_bridge.h"

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

    auto* mapping = new QGroupBox(QCoreApplication::translate("PicturaUi", "Gradient Used for Grayscale Mapping"), this);
    auto* mappingRow = new QHBoxLayout(mapping);
    mappingRow->setSpacing(0);
    sample_ = new QToolButton(mapping);
    sample_->setObjectName(QStringLiteral("gradientMapSample"));
    sample_->setIconSize(kSampleSize);
    sample_->setToolTip(QCoreApplication::translate("PicturaUi", "Click to edit the gradient"));
    presets_ = new QToolButton(mapping);
    presets_->setObjectName(QStringLiteral("gradientMapPresets"));
    presets_->setArrowType(Qt::DownArrow);
    presets_->setPopupMode(QToolButton::InstantPopup);
    presets_->setToolTip(QCoreApplication::translate("PicturaUi", "Click to open Gradient picker"));
    presets_->setFixedHeight(kSampleSize.height() + 8);
    auto* menu = new QMenu(presets_);
    auto& store = PresetStore::current();
    const auto rebuild = [this, menu, &store] {
        menu->clear();
        for (const auto& row : store.rows(PresetType::Gradients)) {
            const auto stops = libraryRamp(row, foreground_, background_);
            if (stops.isEmpty()) continue;
            QPixmap pixmap(32, 16);
            {
                QPainter painter(&pixmap);
                paintGradient(painter, pixmap.rect(), stops);
            }
            connect(menu->addAction(QIcon(pixmap), row.name), &QAction::triggered, this,
                [this, name = row.name, stops, smoothness = row.values.value(QStringLiteral("smoothness")).toDouble(100)] {
                    GradientSpec preset;
                    preset.name = name;
                    preset.stops = stops;
                    preset.smoothness = qBound(0, qRound(smoothness), 100);
                    preset.noise = spec_.noise;
                    setSpec(preset);
                });
        }
    };
    rebuild();
    connect(&store, &PresetStore::changed, this, [rebuild](int type) {
        if (type == int(PresetType::Gradients)) rebuild();
    });
    presets_->setMenu(menu);
    mappingRow->addWidget(sample_, 1);
    mappingRow->addWidget(presets_);
    left->addWidget(mapping);

    auto* options = new QGroupBox(QCoreApplication::translate("PicturaUi", "Gradient Options"), this);
    auto* optionsColumn = new QVBoxLayout(options);
    dither_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Dither"), options);
    dither_->setObjectName(QStringLiteral("dither"));
    reverse_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Reverse"), options);
    reverse_->setObjectName(QStringLiteral("reverse"));
    reverse_->setChecked(gradient_map_reverse(slice(block)));
    // ponytail: the map is not dithered, so Dither stays off-limits; the
    // block's flag is carried through unchanged.
    dither_->setChecked(gradient_map_dither(slice(block)));
    dither_->setEnabled(false);
    dither_->setToolTip(QCoreApplication::translate("PicturaUi", "Dithering is not supported yet"));
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

#include "moc_gradient_map_dialog.cpp"
