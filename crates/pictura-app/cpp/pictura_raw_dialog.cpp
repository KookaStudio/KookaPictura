#include "pictura_raw_dialog.h"

#include "acr/acr_controls.h"
#include "acr/acr_histogram.h"
#include "acr/acr_panels.h"
#include "acr/acr_preview_pane.h"
#include "acr/acr_settings.h"
#include "dialogs.h"
#include "icons.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QList>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

PicturaRawDialog::PicturaRawDialog(PictureView* view, const QString& path,
                                   const QString& initialSettings, QWidget* parent)
    : QDialog(parent), path_(path), settings_(AcrSettings::decode(initialSettings))
{
    setWindowTitle(QStringLiteral("Pictura Raw"));

    auto* outer = new QHBoxLayout(this);

    preview_ = new AcrPreviewPane(view, path, this);
    outer->addWidget(preview_, 1);

    auto* right = new QVBoxLayout;
    histogram_ = new AcrHistogram(this);
    right->addWidget(histogram_);

    tabs_ = new QTabBar(this);
    auto* stack = new QStackedWidget(this);

    struct Tab {
        QString title;
        QString iconId;
        AcrPanel* panel;
    };
    const QList<Tab> panels = {
        {tr("Basic"), QStringLiteral("view.options"), new AcrBasicPanel(&settings_, this)},
        {tr("Tone Curve"), QStringLiteral("window.panels.paths"),
         new AcrToneCurvePanel(&settings_, this)},
        {tr("Detail"), QStringLiteral("tool.sharpen"), new AcrDetailPanel(&settings_, this)},
        {tr("HSL / Grayscale"), QStringLiteral("window.panels.color"),
         new AcrHslPanel(&settings_, this)},
        {tr("Split Toning"), QStringLiteral("window.panels.swatches"),
         new AcrSplitToningPanel(&settings_, this)},
        {tr("Lens Corrections"), QStringLiteral("window.panels.patterns"),
         new AcrLensPanel(&settings_, this)},
        {tr("Effects"), QStringLiteral("tool.blur"), new AcrEffectsPanel(&settings_, this)},
        {tr("Camera Calibration"), QStringLiteral("tool.colorsampler"),
         new AcrCalibrationPanel(&settings_, this)},
        {tr("Presets"), QStringLiteral("layers.fx"), new AcrPresetsPanel(&settings_, this)},
    };
    for (const Tab& entry : panels) {
        tabs_->addTab(pictura::icon(entry.iconId), entry.title);
        stack->addWidget(entry.panel);
        connect(entry.panel, &AcrPanel::changed, this, &PicturaRawDialog::refresh);
    }
    connect(tabs_, &QTabBar::currentChanged, stack, &QStackedWidget::setCurrentIndex);

    right->addWidget(tabs_);
    right->addWidget(stack, 1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &PicturaRawDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &PicturaRawDialog::reject);
    right->addWidget(buttons);

    outer->addLayout(right);
    resize(1100, 680);
    refresh();
}

void PicturaRawDialog::refresh()
{
    const QString text = settings_.encode();
    preview_->setSettingsText(text);
    histogram_->setImage(preview_->image());
}

QString PicturaRawDialog::settingsText() const
{
    return settings_.encode();
}

bool PicturaRawDialog::get(QWidget* parent, PictureView* view, const QString& path,
                           const QString& initialSettings, QString* out)
{
    PicturaRawDialog dialog(view, path, initialSettings, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.settingsText();
    return true;
}

} // namespace pictura
