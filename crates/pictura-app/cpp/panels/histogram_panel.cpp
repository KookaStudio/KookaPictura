#include "histogram_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QRectF>
#include <QtCore/QStringList>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <algorithm>

namespace pictura {

HistogramView::HistogramView(QWidget* parent)
    : QWidget(parent)
{
}

void HistogramView::setSeries(const QVector<Series>& series)
{
    series_ = series;
    update();
}

void HistogramView::clear()
{
    series_.clear();
    update();
}

QColor HistogramView::seriesColorForTest(int index) const
{
    return index >= 0 && index < series_.size() ? series_.at(index).color : QColor();
}

void HistogramView::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(32, 32, 32));
    painter.setPen(QColor(80, 80, 80));
    painter.drawRect(rect().adjusted(0, 0, -1, -1));
    if (series_.isEmpty()) {
        return;
    }
    quint32 peak = 0;
    for (const Series& series : series_) {
        for (quint32 count : series.bins) {
            peak = std::max(peak, count);
        }
    }
    if (peak == 0) {
        return;
    }
    const QRect area = rect().adjusted(1, 1, -1, -1);
    painter.setPen(Qt::NoPen);
    // Additive blend so overlapping channel curves brighten toward white, the
    // way Photoshop's RGB overlay reads.
    painter.setCompositionMode(QPainter::CompositionMode_Plus);
    for (const Series& series : series_) {
        if (series.bins.isEmpty()) {
            continue;
        }
        const double barWidth = double(area.width()) / series.bins.size();
        painter.setBrush(series.color);
        for (int i = 0; i < series.bins.size(); ++i) {
            const double barHeight = double(series.bins[i]) / peak * area.height();
            painter.drawRect(QRectF(area.x() + i * barWidth, area.bottom() + 1 - barHeight,
                                    std::max(1.0, barWidth), barHeight));
        }
    }
}

HistogramPanel::HistogramPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    channel_ = new QComboBox(this);
    channel_->addItem(tr("All Channels"));
    channel_->addItem(tr("Red"));
    channel_->addItem(tr("Green"));
    channel_->addItem(tr("Blue"));
    channel_->addItem(tr("Luminance"));
    layout->addWidget(channel_);
    histogram_ = new HistogramView(this);
    layout->addWidget(histogram_, 1);

    connect(channel_, &QComboBox::currentIndexChanged, this, [this](int) { applyChannel(); });
}

void HistogramPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void HistogramPanel::refresh()
{
    recompute();
    applyChannel();
}

void HistogramPanel::recompute()
{
    bins_ = {};
    if (!view_ || !view_->has_document()) {
        return;
    }
    // Read the coarsest view-pyramid level (always at most 512 px on its long
    // side) instead of the full-resolution image, so the scan cost is
    // independent of document size and no full-resolution image rebuild is
    // triggered by a panel refresh.
    QImage image;
    const int levels = view_->display_level_count();
    if (levels > 0) {
        const int level = levels - 1;
        const QStringList parts = view_->display_level_size(level).split(
            QLatin1Char(' '), Qt::SkipEmptyParts);
        if (parts.size() == 2) {
            image = view_->display_image(level, 0, 0, parts.at(0).toInt(),
                                         parts.at(1).toInt());
        }
    }
    if (image.isNull()) {
        return;
    }
    if (image.width() > 512 || image.height() > 512) {
        image = image.scaled(512, 512, Qt::KeepAspectRatio, Qt::FastTransformation);
    }
    image = image.convertToFormat(QImage::Format_RGB32);
    for (int y = 0; y < image.height(); ++y) {
        const QRgb* line = reinterpret_cast<const QRgb*>(image.constScanLine(y));
        for (int x = 0; x < image.width(); ++x) {
            const QRgb pixel = line[x];
            ++bins_[0][qRed(pixel)];
            ++bins_[1][qGreen(pixel)];
            ++bins_[2][qBlue(pixel)];
            ++bins_[3][qGray(pixel)];
        }
    }
}

void HistogramPanel::applyChannel()
{
    if (!view_) {
        histogram_->clear();
        return;
    }
    static const QColor colors[4] = {
        QColor(230, 90, 90),
        QColor(90, 220, 90),
        QColor(90, 130, 240),
        QColor(220, 220, 220),
    };
    const int index = std::clamp(channel_->currentIndex(), 0, 4);
    QVector<HistogramView::Series> series;
    const auto addChannel = [&](int channelIndex) {
        QVector<quint32> bins(256);
        for (int i = 0; i < 256; ++i) {
            bins[i] = bins_[channelIndex][i];
        }
        series.append(HistogramView::Series{bins, colors[channelIndex]});
    };
    if (index == 0) {
        addChannel(0);
        addChannel(1);
        addChannel(2);
    } else {
        addChannel(index - 1);
    }
    histogram_->setSeries(series);
}

int HistogramPanel::channelIndexForTest() const { return channel_->currentIndex(); }

QString HistogramPanel::channelLabelForTest() const { return channel_->currentText(); }

int HistogramPanel::seriesCountForTest() const { return histogram_->seriesCountForTest(); }

QVector<QColor> HistogramPanel::seriesColorsForTest() const
{
    QVector<QColor> colors;
    for (int i = 0; i < histogram_->seriesCountForTest(); ++i) {
        colors.append(histogram_->seriesColorForTest(i));
    }
    return colors;
}

void HistogramPanel::setChannelIndexForTest(int index) { channel_->setCurrentIndex(index); }

} // namespace pictura
