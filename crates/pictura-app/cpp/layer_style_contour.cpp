#include "layer_style_contour.h"

#include "panels/curve_widget.h"
#include "preset_store.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPointF>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPen>
#include <QtGui/QPaintEvent>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <iterator>

namespace pictura {

namespace {

struct ContourPreset {
    const char* name;
    const char* points;
};

const ContourPreset kPresets[] = {
    {"Linear", "0,0 255,255"},
    {"Cone", "0,0 128,255 255,0"},
    {"Ring", "0,0 64,255 128,0 192,255 255,0"},
    {"Rounded", "0,0 32,96 128,160 224,96 255,0"},
    {"Sawtooth", "0,0 127,255 128,0 255,255"},
};

} // namespace

QStringList builtInContourNames()
{
    QStringList names;
    for (const auto& preset : kPresets) {
        names.push_back(QCoreApplication::translate("ContourEditorDialog", preset.name));
    }
    return names;
}

std::array<qreal, 256> contourLut(const QString& points)
{
    QVector<QPointF> curve;
    for (const QString& pair : points.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        const QStringList xy = pair.split(QLatin1Char(','));
        if (xy.size() != 2) {
            continue;
        }
        bool okX = false;
        bool okY = false;
        const int x = xy[0].toInt(&okX);
        const int y = xy[1].toInt(&okY);
        if (okX && okY && x >= 0 && x <= 255 && y >= 0 && y <= 255) {
            curve.append(QPointF(x / 255.0, y / 255.0));
        }
    }
    std::sort(curve.begin(), curve.end(), [](const QPointF& a, const QPointF& b) {
        return a.x() < b.x();
    });
    if (curve.size() < 2) {
        curve = {QPointF(0.0, 0.0), QPointF(1.0, 1.0)};
    }
    std::array<qreal, 256> lut{};
    for (int i = 0; i < 256; ++i) {
        const qreal x = i / 255.0;
        if (x <= curve.first().x()) {
            lut[i] = qBound(0.0, curve.first().y(), 1.0);
            continue;
        }
        if (x >= curve.last().x()) {
            lut[i] = qBound(0.0, curve.last().y(), 1.0);
            continue;
        }
        for (int s = 0; s + 1 < curve.size(); ++s) {
            if (x >= curve[s].x() && x <= curve[s + 1].x()) {
                const qreal span = curve[s + 1].x() - curve[s].x();
                const qreal t = span > 1e-9 ? (x - curve[s].x()) / span : 1.0;
                lut[i] = qBound(0.0, curve[s].y() + (curve[s + 1].y() - curve[s].y()) * t, 1.0);
                break;
            }
        }
    }
    return lut;
}

ContourSwatch::ContourSwatch(QWidget* parent)
    : QPushButton(parent)
{
    setFixedSize(72, 32);
    setCursor(Qt::PointingHandCursor);
    setToolTip(tr("Edit the contour"));
}

void ContourSwatch::setPointsText(const QString& points)
{
    points_ = points;
    update();
}

void ContourSwatch::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect face = rect().adjusted(0, 0, -1, -1);
    painter.fillRect(face, palette().color(QPalette::Base));
    painter.setPen(palette().color(QPalette::Mid));
    painter.drawRect(face);
    for (int i = 1; i < 4; ++i) {
        const int x = face.left() + face.width() * i / 4;
        const int y = face.top() + face.height() * i / 4;
        painter.drawLine(x, face.top(), x, face.bottom());
        painter.drawLine(face.left(), y, face.right(), y);
    }
    const auto lut = contourLut(points_);
    QPainterPath path;
    for (int i = 0; i < 256; ++i) {
        const qreal x = face.left() + face.width() * i / 255.0;
        const qreal y = face.top() + face.height() * (1.0 - lut[i]);
        if (i == 0) {
            path.moveTo(x, y);
        } else {
            path.lineTo(x, y);
        }
    }
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(QPen(palette().color(QPalette::Text), 1.5));
    painter.drawPath(path);
}

ContourEditorDialog::ContourEditorDialog(const QString& points, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("contourEditor"));
    setWindowTitle(tr("Contour Editor"));
    auto* outer = new QVBoxLayout(this);
    auto* top = new QHBoxLayout;
    top->addWidget(new QLabel(tr("Preset:"), this));
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("contourPreset"));
    preset_->addItem(tr("Custom"));
    auto& store = PresetStore::current();
    const auto rebuild = [this, &store] {
        const QSignalBlocker block(preset_);
        while (preset_->count() > 1) {
            preset_->removeItem(1);
        }
        for (const auto& row : store.rows(PresetType::Contours)) {
            const auto builtIn = row.values.value(QStringLiteral("index")).toInt(-1);
            const QString text = builtIn >= 0 && builtIn < int(std::size(kPresets))
                ? QString::fromLatin1(kPresets[builtIn].points)
                : row.values.value(QStringLiteral("points")).toString();
            if (!text.isEmpty()) {
                preset_->addItem(row.name, text);
            }
        }
        preset_->setCurrentIndex(0);
    };
    rebuild();
    connect(&store, &PresetStore::changed, this, [rebuild](int type) {
        if (type == int(PresetType::Contours)) {
            rebuild();
        }
    });
    top->addWidget(preset_, 1);
    outer->addLayout(top);

    curve_ = new CurveWidget(this);
    curve_->setObjectName(QStringLiteral("contourCurve"));
    curve_->setLinearInterpolation(true);
    curve_->setShowHistogram(false);
    curve_->setShowBaseline(true);
    curve_->setShowRamps(false);
    curve_->setPointsText(points);
    outer->addWidget(curve_, 1);

    auto* buttons = new QHBoxLayout;
    auto* reset = new QPushButton(tr("Reset"), this);
    reset->setObjectName(QStringLiteral("contourReset"));
    buttons->addWidget(reset);
    buttons->addStretch();
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setObjectName(QStringLiteral("contourCancel"));
    auto* ok = new QPushButton(tr("OK"), this);
    ok->setObjectName(QStringLiteral("contourOk"));
    ok->setDefault(true);
    buttons->addWidget(cancel);
    buttons->addWidget(ok);
    outer->addLayout(buttons);

    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(reset, &QPushButton::clicked, this, [this] {
        curve_->setPointsText(QStringLiteral("0,0 255,255"));
        preset_->setCurrentIndex(0);
    });
    connect(preset_, &QComboBox::currentIndexChanged, this, [this](int index) {
        if (index > 0) {
            curve_->setPointsText(preset_->itemData(index).toString());
        }
    });
    connect(curve_, &CurveWidget::curveChanged, this, [this] {
        const QSignalBlocker block(preset_);
        preset_->setCurrentIndex(0);
    });
    resize(360, 420);
}

QString ContourEditorDialog::pointsText() const
{
    return curve_ ? curve_->pointsText() : QString();
}

} // namespace pictura

#include "moc_layer_style_contour.cpp"
