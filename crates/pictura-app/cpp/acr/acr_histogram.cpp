#include "acr_histogram.h"
#include <QtCore/QCoreApplication>

#include "panels/histogram_panel.h"

#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QHBoxLayout>
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

    defaultReadout_ = QCoreApplication::translate("PicturaUi", "R: —  G: —  B: —");
    readout_ = new QLabel(defaultReadout_, this);
    auto* row = new QHBoxLayout;
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(readout_);
    row->addStretch(1);
    auto* exposure = new QVBoxLayout;
    exposure->setContentsMargins(0, 0, 0, 0);
    exposure->setSpacing(0);
    exposureTop_ = new QLabel(this);
    exposureTop_->setObjectName(QStringLiteral("acrExposureTop"));
    exposureTop_->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    exposureBottom_ = new QLabel(this);
    exposureBottom_->setObjectName(QStringLiteral("acrExposureBottom"));
    exposureBottom_->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    exposure->addWidget(exposureTop_);
    exposure->addWidget(exposureBottom_);
    row->addLayout(exposure);
    layout->addLayout(row);
}

void AcrHistogram::setExposureInfo(const QString& top, const QString& bottom)
{
    exposureTop_->setText(top);
    exposureTop_->setVisible(!top.isEmpty());
    exposureBottom_->setText(bottom);
    exposureBottom_->setVisible(!bottom.isEmpty());
}

void AcrHistogram::setPickerColor(int r, int g, int b)
{
    if (r < 0 || g < 0 || b < 0) {
        pickerActive_ = false;
        readout_->setText(defaultReadout_);
        return;
    }
    pickerActive_ = true;
    readout_->setText(QCoreApplication::translate("PicturaUi", "R: %1  G: %2  B: %3")
                          .arg(r)
                          .arg(g)
                          .arg(b));
}

void AcrHistogram::clear()
{
    view_->clear();
    pickerActive_ = false;
    defaultReadout_ = QCoreApplication::translate("PicturaUi", "R: —  G: —  B: —");
    readout_->setText(defaultReadout_);
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
        defaultReadout_ = QCoreApplication::translate("PicturaUi", "R: %1  G: %2  B: %3")
                              .arg(sumR / count)
                              .arg(sumG / count)
                              .arg(sumB / count);
        if (!pickerActive_) {
            readout_->setText(defaultReadout_);
        }
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
