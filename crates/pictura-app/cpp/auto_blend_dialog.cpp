#include "auto_blend_dialog.h"

#include <QtGui/QLinearGradient>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr QSize kPreviewSize(150, 72);
const QColor kFrame(16, 18, 24);

// A small landscape: sky, a hill and water, shifted by `tone` so frames differ.
void drawLandscape(QPainter& p, const QRect& r, int tone)
{
    p.save();
    p.setClipRect(r);
    QLinearGradient sky(r.topLeft(), r.bottomLeft());
    sky.setColorAt(0.0, QColor(96 + tone, 160, 222));
    sky.setColorAt(1.0, QColor(196, 226 - tone, 240));
    p.fillRect(r, sky);
    p.setPen(Qt::NoPen);
    p.setBrush(QColor(52, 112 + tone, 58));
    p.drawEllipse(QRect(r.left() - r.width() / 3, r.top() + r.height() / 3, r.width(), r.height()));
    p.setBrush(QColor(36, 110, 150 + tone));
    p.drawRect(r.left(), r.top() + r.height() * 3 / 4, r.width(), r.height());
    p.restore();
    p.setPen(kFrame);
    p.setBrush(Qt::NoBrush);
    p.drawRect(r.adjusted(0, 0, -1, -1));
}

// A five-petal flower on foliage, blurred-looking when `soft`.
void drawFlower(QPainter& p, const QRect& r, bool soft)
{
    p.save();
    p.setClipRect(r);
    p.fillRect(r, soft ? QColor(74, 104, 62) : QColor(38, 84, 44));
    p.translate(QRectF(r).center());
    p.setPen(Qt::NoPen);
    p.setBrush(soft ? QColor(232, 150, 96) : QColor(240, 118, 34));
    const qreal petal = r.height() * 0.26;
    for (int i = 0; i < 5; ++i) {
        p.rotate(72.0);
        p.drawEllipse(QRectF(-petal / 2, -petal * 1.5, petal, petal * 1.3));
    }
    p.setBrush(QColor(248, 206, 70));
    p.drawEllipse(QRectF(-petal / 2.5, -petal / 2.5, petal / 1.25, petal / 1.25));
    p.restore();
    p.setPen(kFrame);
    p.setBrush(Qt::NoBrush);
    p.drawRect(r.adjusted(0, 0, -1, -1));
}

QPixmap blankPreview()
{
    QPixmap pixmap(kPreviewSize);
    pixmap.fill(Qt::transparent);
    return pixmap;
}

// Original artwork drawn in code: three overlapping frames with overlap marks.
QPixmap panoramaPreview(const QColor& ink)
{
    QPixmap pixmap = blankPreview();
    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing);
    drawLandscape(p, QRect(2, 12, 64, 42), 0);
    drawLandscape(p, QRect(84, 4, 64, 42), 22);
    drawLandscape(p, QRect(42, 20, 64, 42), -14);
    p.setPen(ink);
    for (const int x : {44, 86}) {
        p.drawLine(x, 68, x + 18, 68);
        p.drawLine(x, 68, x + 3, 65);
        p.drawLine(x, 68, x + 3, 71);
        p.drawLine(x + 18, 68, x + 15, 65);
        p.drawLine(x + 18, 68, x + 15, 71);
    }
    return pixmap;
}

// Original artwork drawn in code: a stack of frames becoming one sharp frame.
QPixmap stackPreview(const QColor& ink)
{
    QPixmap pixmap = blankPreview();
    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing);
    drawFlower(p, QRect(2, 4, 54, 44), true);
    drawFlower(p, QRect(9, 12, 54, 44), true);
    drawFlower(p, QRect(16, 20, 54, 44), true);
    p.setPen(ink);
    p.drawLine(76, 40, 90, 40);
    p.drawLine(90, 40, 86, 36);
    p.drawLine(90, 40, 86, 44);
    drawFlower(p, QRect(94, 18, 54, 44), false);
    return pixmap;
}

QLabel* previewLabel(const QPixmap& pixmap, const QString& name, QWidget* parent)
{
    auto* label = new QLabel(parent);
    label->setObjectName(name);
    label->setPixmap(pixmap);
    return label;
}

} // namespace

AutoBlendDialog::AutoBlendDialog(bool allowFill, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("autoBlendDialog"));
    setWindowTitle(tr("Auto-Blend Layers"));
    const QColor ink = palette().color(QPalette::WindowText);

    auto* method = new QGroupBox(tr("Blend Method"), this);
    auto* methodLayout = new QVBoxLayout(method);
    auto* panorama = new QRadioButton(tr("Panorama"), method);
    panorama->setObjectName(QStringLiteral("autoBlendPanorama"));
    panorama->setChecked(true);
    stack_ = new QRadioButton(tr("Stack Images"), method);
    stack_->setObjectName(QStringLiteral("autoBlendStack"));
    methodLayout->addWidget(panorama);
    methodLayout->addWidget(
        previewLabel(panoramaPreview(ink), QStringLiteral("autoBlendPanoramaPreview"), method));
    methodLayout->addWidget(stack_);
    methodLayout->addWidget(
        previewLabel(stackPreview(ink), QStringLiteral("autoBlendStackPreview"), method));

    seamless_ = new QCheckBox(tr("Seamless Tones and Colors"), this);
    seamless_->setObjectName(QStringLiteral("autoBlendSeamless"));
    fill_ = new QCheckBox(tr("Content Aware Fill Transparent Areas"), this);
    fill_->setObjectName(QStringLiteral("autoBlendFill"));
    fill_->setEnabled(allowFill);

    auto* left = new QVBoxLayout;
    left->addWidget(method);
    left->addWidget(seamless_);
    left->addWidget(fill_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         Qt::Vertical, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* layout = new QHBoxLayout(this);
    layout->addLayout(left);
    layout->addWidget(buttons);
}

bool AutoBlendDialog::stackImages() const
{
    return stack_->isChecked();
}

bool AutoBlendDialog::seamless() const
{
    return seamless_->isChecked();
}

bool AutoBlendDialog::fillTransparent() const
{
    return fill_->isEnabled() && fill_->isChecked();
}

} // namespace pictura
