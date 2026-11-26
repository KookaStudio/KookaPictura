#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QWidget>

#include <functional>
#include <vector>

class QComboBox;
class QLabel;
class QVBoxLayout;

namespace pictura {

class CurveWidget;
class HistogramView;
class PictureView;

// An adjustment's controls, built from the engine's descriptor page (the rows
// `adjustment_page` / `image_adjustment_page` return): sliders with number
// fields, checks, menus, a colour swatch, the Curves editor with its channel
// menu, and a group menu that switches which controls show. Shared by the
// Properties panel and the Image > Adjustments dialogs; the owner applies the
// edits it reports.
class AdjustmentControls : public QWidget {
    Q_OBJECT

public:
    explicit AdjustmentControls(QWidget* parent = nullptr);

    // Rebuild for `page` when its set of controls differs from the built one;
    // otherwise reload the values (never while `holdValues`).
    void setPage(const QStringList& page, bool holdValues = false);
    void refreshThresholdHistogram(const PictureView& view, const QString& path = {});
    bool hasThresholdHistogram() const { return thresholdHistogram_ != nullptr; }
    QString title() const { return title_; }
    // Where the Curves editor reads `channel`'s points (`"x,y x,y …"`).
    void setCurveSource(std::function<QString(int)> source) { curveSource_ = std::move(source); }
    QWidget* controlForTest(const QString& key) const;

signals:
    void valueChanged(const QString& key, double value);
    void curveChanged(int channel, const QString& points);
    // A gesture ended (a slider release, toggle, menu, or colour pick).
    void gestureEnded();

private:
    struct Row {
        QString key;
        int group = -1;
        QWidget* widget = nullptr;
        QWidget* control = nullptr;
        std::function<void(double)> load;
    };

    void build(const QStringList& page);
    void load(const QStringList& page);
    void clear();
    void insert(QWidget* widget);
    void addSlider(const QStringList& cells, bool threshold);
    void addCheck(const QStringList& cells);
    void addChoice(const QStringList& cells);
    void addColor(const QStringList& cells);
    void addCurves();
    void showGroup(int group);
    void loadCurve();

    QVBoxLayout* layout_ = nullptr;
    QString signature_;
    QString title_;
    bool loading_ = false;
    std::vector<Row> rows_;
    QComboBox* groups_ = nullptr;
    CurveWidget* curve_ = nullptr;
    HistogramView* thresholdHistogram_ = nullptr;
    QComboBox* curveChannel_ = nullptr;
    std::function<QString(int)> curveSource_;
};

} // namespace pictura
