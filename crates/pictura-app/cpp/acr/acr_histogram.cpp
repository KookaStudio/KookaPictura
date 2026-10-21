#include "acr_histogram.h"

#include "panels/histogram_panel.h"

#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

std::array<int, 256> acrLumaBins(const QImage& image)
{
    std::array<int, 256> bins{};
    if (image.isNull()) {
        return bins;
    }
    const QImage src = image.convertToFormat(QImage::Format_RGB32);
    for (int y = 0; y < src.height(); ++y) {
        const QRgb* line = reinterpret_cast<const QRgb*>(src.constScanLine(y));
        for (int x = 0; x < src.width(); ++x) {
            bins[qGray(line[x])] += 1;
        }
    }
    return bins;
}

AcrHistogram::AcrHistogram(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(2);
    view_ = new HistogramView(this);
    view_->setMinimumHeight(80);
    layout->addWidget(view_);
    readout_ = new QLabel(QStringLiteral("R: —  G: —  B: —"), this);
    layout->addWidget(readout_);
}

void AcrHistogram::clear()
{
    view_->clear();
    readout_->setText(QStringLiteral("R: —  G: —  B: —"));
}

void AcrHistogram::setImage(const QImage& image)
{
    if (image.isNull()) {
        clear();
        return;
    }
    QImage src = image;
    if (src.width() > 512 || src.height() > 512) {
        src = src.scaled(512, 512, Qt::KeepAspectRatio, Qt::FastTransformation);
    }
    src = src.convertToFormat(QImage::Format_RGB32);

    QVector<quint32> red(256, 0);
    QVector<quint32> green(256, 0);
    QVector<quint32> blue(256, 0);
    quint64 sumR = 0;
    quint64 sumG = 0;
    quint64 sumB = 0;
    quint64 count = 0;
    for (int y = 0; y < src.height(); ++y) {
        const QRgb* line = reinterpret_cast<const QRgb*>(src.constScanLine(y));
        for (int x = 0; x < src.width(); ++x) {
            const QRgb pixel = line[x];
            ++red[qRed(pixel)];
            ++green[qGreen(pixel)];
            ++blue[qBlue(pixel)];
            sumR += qRed(pixel);
            sumG += qGreen(pixel);
            sumB += qBlue(pixel);
            ++count;
        }
    }

    QVector<HistogramView::Series> series;
    series.append({red, QColor(230, 90, 90)});
    series.append({green, QColor(90, 220, 90)});
    series.append({blue, QColor(90, 130, 240)});
    view_->setSeries(series);

    if (count > 0) {
        readout_->setText(QStringLiteral("R: %1  G: %2  B: %3")
                              .arg(sumR / count)
                              .arg(sumG / count)
                              .arg(sumB / count));
    }
}

int AcrHistogram::seriesCountForTest() const
{
    return view_->seriesCountForTest();
}

QString AcrHistogram::readoutForTest() const
{
    return readout_->text();
}

} // namespace pictura

#include "moc_acr_histogram.cpp"
