#include "gradient_fill_dialog.h"

#include "dialogs.h"
#include "panels/numeric_field.h"

#include <QtCore/QStringList>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

QString formatStops(const GradientStops& stops)
{
    QStringList out;
    for (const GradientColorStop& stop : stops) {
        out.push_back(QStringLiteral("%1:%2")
                          .arg(stop.location)
                          .arg(stop.color.rgb() & 0xffffff, 6, 16, QLatin1Char('0')));
    }
    return out.join(QLatin1Char(','));
}

GradientStops parseStops(const QString& text)
{
    GradientStops stops;
    for (const QString& item : text.split(QLatin1Char(','), Qt::SkipEmptyParts)) {
        const qsizetype colon = item.indexOf(QLatin1Char(':'));
        if (colon <= 0) {
            continue;
        }
        bool ok = false;
        const int location = item.left(colon).toInt(&ok);
        const uint rgb = item.mid(colon + 1).toUInt(&ok, 16);
        if (!ok) {
            continue;
        }
        stops.push_back({location, QColor::fromRgb(rgb)});
    }
    return stops;
}

} // namespace

GradientFillDialog::GradientFillDialog(const QString& payload, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("gradientFillDialog"));
    setWindowTitle(tr("Gradient Fill"));

    const QStringList parts = payload.split(QLatin1Char('|'));
    const QString kind = parts.isEmpty() ? QStringLiteral("linear") : parts.at(0);
    const double angle = parts.size() > 1 ? parts.at(1).toDouble() : 0.0;
    const double scale = parts.size() > 2 ? parts.at(2).toDouble() : 100.0;
    const bool reverse = parts.size() > 3 && parts.at(3) == QLatin1String("1");
    GradientStops stops = parts.size() > 4 ? parseStops(parts.at(4)) : GradientStops{};
    if (stops.size() < 2) {
        stops = {{0, QColor(Qt::black)}, {4096, QColor(Qt::white)}};
    }
    spec_.name = QStringLiteral("Gradient");
    spec_.type = GradientSpec::Solid;
    spec_.stops = stops;
    spec_.smoothness = 100;

    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(10);

    sample_ = new QToolButton(this);
    sample_->setObjectName(QStringLiteral("fillGradientSample"));
    sample_->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    sample_->setText(tr("Gradient"));
    sample_->setIconSize(QSize(180, 24));
    sample_->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    connect(sample_, &QToolButton::clicked, this, &GradientFillDialog::openEditor);
    outer->addWidget(sample_);

    auto* form = new QFormLayout;
    style_ = new QComboBox(this);
    style_->setObjectName(QStringLiteral("fillGradientStyle"));
    style_->addItem(tr("Linear"), QStringLiteral("linear"));
    style_->addItem(tr("Radial"), QStringLiteral("radial"));
    style_->addItem(tr("Angled"), QStringLiteral("angled"));
    style_->addItem(tr("Reflected"), QStringLiteral("reflected"));
    style_->addItem(tr("Diamond"), QStringLiteral("diamond"));
    const int styleIndex = style_->findData(kind);
    style_->setCurrentIndex(styleIndex >= 0 ? styleIndex : 0);
    form->addRow(tr("Style:"), style_);

    NumericFieldConfig angleConfig;
    angleConfig.minimum = 0.0;
    angleConfig.maximum = 360.0;
    angleConfig.decimals = 1;
    angleConfig.suffix = QStringLiteral(" °");
    angleConfig.objectName = QStringLiteral("fillGradientAngle");
    angle_ = new NumericField(QString(), angleConfig, this);
    angle_->setValue(angle);
    form->addRow(tr("Angle:"), angle_);

    NumericFieldConfig scaleConfig;
    scaleConfig.minimum = 1.0;
    scaleConfig.maximum = 1000.0;
    scaleConfig.decimals = 0;
    scaleConfig.suffix = QStringLiteral(" %");
    scaleConfig.objectName = QStringLiteral("fillGradientScale");
    scale_ = new NumericField(QString(), scaleConfig, this);
    scale_->setValue(scale);
    form->addRow(tr("Scale:"), scale_);

    reverse_ = new QCheckBox(tr("Reverse"), this);
    reverse_->setObjectName(QStringLiteral("fillGradientReverse"));
    reverse_->setChecked(reverse);
    connect(reverse_, &QCheckBox::toggled, this, [this] { updateSample(); });
    form->addRow(QString(), reverse_);

    auto* dither = new QCheckBox(tr("Dither"), this);
    dither->setObjectName(QStringLiteral("fillGradientDither"));
    dither->setEnabled(false);
    dither->setToolTip(tr("Dither — not implemented yet"));
    form->addRow(QString(), dither);

    auto* align = new QCheckBox(tr("Align With Layer"), this);
    align->setObjectName(QStringLiteral("fillGradientAlign"));
    align->setChecked(true);
    align->setEnabled(false);
    align->setToolTip(tr("Align With Layer — not implemented yet"));
    form->addRow(QString(), align);
    outer->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    outer->addWidget(buttons);

    updateSample();
}

void GradientFillDialog::updateSample()
{
    QPixmap pixmap(180, 24);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    GradientStops stops = resolvedStops(spec_);
    if (reverse_->isChecked()) {
        for (GradientColorStop& stop : stops) {
            stop.location = 4096 - stop.location;
        }
        std::sort(stops.begin(), stops.end(),
                  [](const GradientColorStop& a, const GradientColorStop& b) {
                      return a.location < b.location;
                  });
    }
    paintGradient(painter, pixmap.rect(), stops);
    painter.end();
    sample_->setIcon(QIcon(pixmap));
}

void GradientFillDialog::openEditor()
{
    const GradientSpec before = spec_;
    GradientEditorDialog dialog(spec_, Qt::black, Qt::white, this);
    connect(&dialog, &GradientEditorDialog::gradientChanged, this, [this, &dialog] {
        spec_ = dialog.spec();
        updateSample();
    });
    if (runDialog(dialog, this) == QDialog::Accepted) {
        spec_ = dialog.spec();
    } else {
        spec_ = before;
    }
    updateSample();
}

QString GradientFillDialog::payload() const
{
    return QStringLiteral("%1|%2|%3|%4|%5")
        .arg(style_->currentData().toString())
        .arg(angle_->value())
        .arg(scale_->value())
        .arg(reverse_->isChecked() ? 1 : 0)
        .arg(formatStops(spec_.stops));
}

bool GradientFillDialog::getFill(const QString& initial, QWidget* parent, QString* out)
{
    GradientFillDialog dialog(initial, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    if (out) {
        *out = dialog.payload();
    }
    return true;
}

} // namespace pictura

#include "moc_gradient_fill_dialog.cpp"
