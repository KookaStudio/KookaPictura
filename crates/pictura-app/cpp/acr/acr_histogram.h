#pragma once

// The ACR histogram: the RGB channels of the previewed image over a dark plot,
// with a per-channel readout beneath.

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <array>

class QLabel;

namespace pictura {

class HistogramView;

// A 256-bin luminance histogram of `image`, used by the histogram and the tone
// curve's parametric graph.
std::array<int, 256> acrLumaBins(const QImage& image);

class AcrHistogram : public QWidget {
    Q_OBJECT

public:
    explicit AcrHistogram(QWidget* parent = nullptr);

    void setImage(const QImage& image);
    void clear();

    int seriesCountForTest() const;
    QString readoutForTest() const;

private:
    HistogramView* view_ = nullptr;
    QLabel* readout_ = nullptr;
};

} // namespace pictura
