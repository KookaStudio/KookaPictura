#pragma once

// The ACR histogram: the RGB channels of the previewed image over a dark plot,
// with a per-channel readout beneath.

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QLabel;

namespace pictura {

class HistogramView;

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
