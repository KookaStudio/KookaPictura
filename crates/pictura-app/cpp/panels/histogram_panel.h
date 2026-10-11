#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include <array>

class QComboBox;

namespace pictura {

class PictureView;

// Custom-painted 256-bin histogram; one or more overlaid channel curves.
class HistogramView : public QWidget {
public:
    struct Series {
        QVector<quint32> bins;
        QColor color;
    };

    explicit HistogramView(QWidget* parent = nullptr);

    void setSeries(const QVector<Series>& series);
    void clear();

    int seriesCountForTest() const { return series_.size(); }
    QColor seriesColorForTest(int index) const;

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QVector<Series> series_;
};

class HistogramPanel : public QWidget {
    Q_OBJECT

public:
    explicit HistogramPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    int channelIndexForTest() const;
    QString channelLabelForTest() const;
    int seriesCountForTest() const;
    QVector<QColor> seriesColorsForTest() const;
    void setChannelIndexForTest(int index);

private:
    void recompute();
    void applyChannel();

    PictureView* view_ = nullptr;
    HistogramView* histogram_ = nullptr;
    QComboBox* channel_ = nullptr;
    std::array<std::array<quint32, 256>, 4> bins_{};
};

} // namespace pictura
