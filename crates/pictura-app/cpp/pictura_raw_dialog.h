#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

#include "acr/acr_panels.h"
#include "acr/acr_settings.h"

#include <functional>
#include <optional>

class QLabel;
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
    // A camera raw open (#400): previews `view`'s pending raw source, with an
    // absolute white balance that resets to `asShot`.
    PicturaRawDialog(PictureView* view, const AcrAsShot& asShot, const QString& initialSettings,
                     QWidget* parent = nullptr);

    // The full settings text with the edited values written back.
    QString settingsText() const;

    // A raw open's Done (#401): save the settings, open nothing. The dialog
    // is rejected with this set.
    bool saveOnlyRequested() const { return saveOnly_; }

    // How a raw open's Save Image (#402) runs its nested dialogs (Save
    // Options, the replace question): true when accepted. The coordinator
    // routes them through its import-dialog runner.
    void setDialogRunner(std::function<bool(QDialog&)> runner) { runner_ = std::move(runner); }

    // Run the dialog modally; on OK fill `out` with the edited settings text.
    static bool get(QWidget* parent, PictureView* view, const QString& path,
                    const QString& initialSettings, QString* out);

    QTabBar* tabBarForTest() const { return tabs_; }
    AcrPreviewPane* previewPaneForTest() const { return preview_; }
    AcrHistogram* histogramForTest() const { return histogram_; }

private:
    PicturaRawDialog(PictureView* view, const QString& path, std::optional<AcrAsShot> asShot,
                     const QString& initialSettings, QWidget* parent);

    void refresh();
    void render();
    void saveImage();

    PictureView* view_ = nullptr;
    QString path_;
    AcrSettings settings_;
    QTabBar* tabs_ = nullptr;
    AcrPreviewPane* preview_ = nullptr;
    AcrHistogram* histogram_ = nullptr;
    AcrToneCurvePanel* toneCurve_ = nullptr;
    QTimer* refreshTimer_ = nullptr;
    QLabel* saveStatus_ = nullptr;
    std::function<bool(QDialog&)> runner_;
    bool renderPending_ = false;
    bool saveOnly_ = false;
};

} // namespace pictura
