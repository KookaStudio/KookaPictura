#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

#include "acr/acr_settings.h"

class QTabBar;
class QTimer;

namespace pictura {

class AcrHistogram;
class AcrPreviewPane;
class AcrToneCurvePanel;
class PictureView;

// `Filter > Pictura Raw…`: the tabbed ACR dialog — a preview pane, a histogram,
// an icon/text tab strip of the nine panels, and OK / Cancel, over the shared
// settings map carried as text.
class PicturaRawDialog : public QDialog {
    Q_OBJECT

public:
    PicturaRawDialog(PictureView* view, const QString& path, const QString& initialSettings,
                     QWidget* parent = nullptr);

    // The full settings text with the edited values written back.
    QString settingsText() const;

    // Run the dialog modally; on OK fill `out` with the edited settings text.
    static bool get(QWidget* parent, PictureView* view, const QString& path,
                    const QString& initialSettings, QString* out);

    QTabBar* tabBarForTest() const { return tabs_; }
    AcrPreviewPane* previewPaneForTest() const { return preview_; }
    AcrHistogram* histogramForTest() const { return histogram_; }

private:
    void refresh();
    void render();

    QString path_;
    AcrSettings settings_;
    QTabBar* tabs_ = nullptr;
    AcrPreviewPane* preview_ = nullptr;
    AcrHistogram* histogram_ = nullptr;
    AcrToneCurvePanel* toneCurve_ = nullptr;
    QTimer* refreshTimer_ = nullptr;
    bool renderPending_ = false;
};

} // namespace pictura
