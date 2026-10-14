#include "acr_preview_pane.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QRect>
#include <QtGui/QPainter>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
const int kBottomRowHeight = 28;
const int kPreviewEdge = 512;
} // namespace

AcrPreviewPane::AcrPreviewPane(PictureView* view, const QString& path, QWidget* parent)
    : QWidget(parent), view_(view), path_(path)
{
    setMinimumSize(240, 200);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->addStretch(1);

    auto* row = new QHBoxLayout;
    row->setContentsMargins(4, 0, 4, 4);
    zoomLabel_ = new QLabel(QStringLiteral("Fit"), this);
    row->addWidget(zoomLabel_);
    row->addStretch(1);

    shadowToggle_ = new QToolButton(this);
    shadowToggle_->setCheckable(true);
    shadowToggle_->setText(tr("Shadows"));
    shadowToggle_->setToolTip(tr("Show shadow clipping"));
    highlightToggle_ = new QToolButton(this);
    highlightToggle_->setCheckable(true);
    highlightToggle_->setText(tr("Highlights"));
    highlightToggle_->setToolTip(tr("Show highlight clipping"));
    row->addWidget(shadowToggle_);
    row->addWidget(highlightToggle_);
    layout->addLayout(row);

    connect(shadowToggle_, &QToolButton::toggled, this, [this](bool) { update(); });
    connect(highlightToggle_, &QToolButton::toggled, this, [this](bool) { update(); });
}

bool AcrPreviewPane::shadowClipping() const
{
    return shadowToggle_ && shadowToggle_->isChecked();
}

bool AcrPreviewPane::highlightClipping() const
{
    return highlightToggle_ && highlightToggle_->isChecked();
}

void AcrPreviewPane::setSettingsText(const QString& text)
{
    image_ = view_ ? view_->pictura_raw_preview(path_, text, kPreviewEdge) : QImage();
    zoomLabel_->setText(image_.isNull() ? tr("—") : tr("Fit"));
    update();
}

QImage AcrPreviewPane::applyClipping(const QImage& image, bool shadow, bool highlight)
{
    if (image.isNull() || (!shadow && !highlight)) {
        return image;
    }
    QImage out = image.convertToFormat(QImage::Format_ARGB32);
    for (int y = 0; y < out.height(); ++y) {
        auto* line = reinterpret_cast<QRgb*>(out.scanLine(y));
        for (int x = 0; x < out.width(); ++x) {
            const QRgb pixel = line[x];
            const int r = qRed(pixel);
            const int g = qGreen(pixel);
            const int b = qBlue(pixel);
            if (shadow && r <= 2 && g <= 2 && b <= 2) {
                line[x] = qRgb(0, 0, 255);
            } else if (highlight && r >= 253 && g >= 253 && b >= 253) {
                line[x] = qRgb(255, 0, 0);
            }
        }
    }
    return out;
}

void AcrPreviewPane::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect area = rect().adjusted(0, 0, 0, -kBottomRowHeight);
    painter.fillRect(area, QColor(70, 70, 70));
    const int tile = 12;
    for (int y = area.top(); y <= area.bottom(); y += tile) {
        for (int x = area.left(); x <= area.right(); x += tile) {
            const bool dark = ((x / tile) + (y / tile)) % 2 == 0;
            painter.fillRect(QRect(x, y, tile, tile), dark ? QColor(90, 90, 90)
                                                            : QColor(120, 120, 120));
        }
    }
    if (image_.isNull() || area.isEmpty()) {
        return;
    }
    const QImage shown = applyClipping(image_, shadowClipping(), highlightClipping());
    const QSize scaled = shown.size().scaled(area.size(), Qt::KeepAspectRatio);
    const QRect target(area.center().x() - scaled.width() / 2,
                       area.center().y() - scaled.height() / 2, scaled.width(), scaled.height());
    painter.drawImage(target, shown);
}

} // namespace pictura
