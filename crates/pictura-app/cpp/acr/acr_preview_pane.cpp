#include "acr_preview_pane.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "theme.h"

#include <QtCore/QRect>
#include <QtGui/QPainter>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

namespace {
const int kBottomRowHeight = 28;
const int kPreviewEdge = 512;
const double kZoomSteps[] = {0.0625, 0.125, 0.25, 0.333, 0.5, 0.667, 1.0, 2.0, 4.0};
const int kZoomStepCount = 9;
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

    zoomOut_ = new QToolButton(this);
    zoomOut_->setObjectName(QStringLiteral("acrZoomOut"));
    zoomOut_->setText(QStringLiteral("−"));
    zoomIn_ = new QToolButton(this);
    zoomIn_->setObjectName(QStringLiteral("acrZoomIn"));
    zoomIn_->setText(QStringLiteral("+"));
    zoomEdit_ = new QLineEdit(this);
    zoomEdit_->setObjectName(QStringLiteral("acrZoomEdit"));
    zoomEdit_->setFixedWidth(58);
    zoomEdit_->setAlignment(Qt::AlignCenter);
    row->addWidget(zoomOut_);
    row->addWidget(zoomIn_);
    row->addWidget(zoomEdit_);
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
    connect(zoomOut_, &QToolButton::clicked, this, [this] { stepZoom(-1); });
    connect(zoomIn_, &QToolButton::clicked, this, [this] { stepZoom(1); });
    connect(zoomEdit_, &QLineEdit::editingFinished, this, &AcrPreviewPane::commitZoomEdit);

    refreshZoomEdit();
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
    update();
}

void AcrPreviewPane::setZoom(double zoom)
{
    if (zoom <= 0.0) {
        fit_ = true;
    } else {
        fit_ = false;
        zoom_ = qBound(kZoomSteps[0], zoom, kZoomSteps[kZoomStepCount - 1]);
    }
    refreshZoomEdit();
    update();
}

void AcrPreviewPane::stepZoom(int direction)
{
    const double current = fit_ ? 0.0 : zoom_;
    if (direction > 0) {
        for (double step : kZoomSteps) {
            if (step > current + 1e-6) {
                setZoom(step);
                return;
            }
        }
    } else {
        for (int i = kZoomStepCount - 1; i >= 0; --i) {
            if (kZoomSteps[i] < current - 1e-6) {
                setZoom(kZoomSteps[i]);
                return;
            }
        }
    }
}

void AcrPreviewPane::commitZoomEdit()
{
    const QString text = zoomEdit_->text().trimmed();
    if (text.compare(QStringLiteral("fit"), Qt::CaseInsensitive) == 0) {
        setZoom(0.0);
        return;
    }
    QString number = text;
    number.remove(QLatin1Char('%'));
    bool ok = false;
    const double percent = number.toDouble(&ok);
    if (ok && percent > 0.0) {
        setZoom(percent / 100.0);
    } else {
        refreshZoomEdit();
    }
}

void AcrPreviewPane::refreshZoomEdit()
{
    zoomEdit_->setText(fit_ ? tr("Fit")
                            : QStringLiteral("%1%").arg(qRound(zoom_ * 1000.0) / 10.0));
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
    painter.fillRect(area, Theme::workspaceColor());
    if (image_.isNull() || area.isEmpty()) {
        return;
    }
    const QImage shown = applyClipping(image_, shadowClipping(), highlightClipping());
    QSize scaled;
    if (fit_) {
        scaled = shown.size().scaled(area.size(), Qt::KeepAspectRatio);
    } else {
        scaled = QSize(qMax(1, qRound(shown.width() * zoom_)),
                       qMax(1, qRound(shown.height() * zoom_)));
    }
    const QRect target(area.center().x() - scaled.width() / 2,
                       area.center().y() - scaled.height() / 2, scaled.width(), scaled.height());
    painter.drawImage(target, shown);
}

} // namespace pictura

#include "moc_acr_preview_pane.cpp"
