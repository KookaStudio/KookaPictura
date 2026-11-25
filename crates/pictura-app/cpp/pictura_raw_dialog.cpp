#include "pictura_raw_dialog.h"

#include "acr/acr_controls.h"
#include "acr/acr_histogram.h"
#include "acr/acr_panels.h"
#include "acr/acr_preview_pane.h"
#include "acr/acr_settings.h"
#include "dialogs.h"
#include "icons.h"
#include "raw_save_options_dialog.h"

#include "api/filters_bridge.h"
#include "api/view_bridge.h"

#include <QtCore/QList>
#include <QtCore/QMap>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSizeGrip>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

PicturaRawDialog::PicturaRawDialog(PictureView* view, const QString& path,
                                   const QString& initialSettings, QWidget* parent)
    : PicturaRawDialog(view, path, std::nullopt, initialSettings, parent)
{
}

PicturaRawDialog::PicturaRawDialog(PictureView* view, const AcrAsShot& asShot,
                                   const QString& initialSettings, QWidget* parent)
    : PicturaRawDialog(view, QString(), asShot, initialSettings, parent)
{
}

PicturaRawDialog::PicturaRawDialog(PictureView* view, const QString& path,
                                   std::optional<AcrAsShot> asShot,
                                   const QString& initialSettings, QWidget* parent)
    : QDialog(parent), view_(view), path_(path),
      settings_(AcrSettings::decode(initialSettings)), asShot_(asShot)
{
    setObjectName(QStringLiteral("picturaRawDialog"));
    setWindowTitle(QStringLiteral("Pictura Raw"));

    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(8, 8, 8, 4);
    root->setSpacing(4);

    auto* outer = new QHBoxLayout;
    outer->setSpacing(14);

    preview_ = new AcrPreviewPane(view, path, this);
    preview_->setRawSource(asShot.has_value());
    outer->addWidget(preview_, 1);

    auto* right = new QVBoxLayout;
    right->setContentsMargins(0, 0, 0, 0);
    right->setSpacing(6);
    histogram_ = new AcrHistogram(this);
    right->addWidget(histogram_);
    connect(preview_, &AcrPreviewPane::picked, this, [this](int r, int g, int b) {
        histogram_->setPickerColor(r, g, b);
    });

    tabs_ = new QTabBar(this);
    tabs_->setObjectName(QStringLiteral("acrTabBar"));
    tabs_->setDrawBase(false);
    tabs_->setExpanding(true);
    tabs_->setUsesScrollButtons(false);
    tabs_->setIconSize(QSize(16, 16));
    stack_ = new QStackedWidget(this);

    struct Tab {
        QString title;
        QString iconId;
    };
    const QList<Tab> tabDefs = {
        {tr("Basic"), QStringLiteral("view.options")},
        {tr("Tone Curve"), QStringLiteral("window.panels.paths")},
        {tr("Detail"), QStringLiteral("tool.sharpen")},
        {tr("HSL / Grayscale"), QStringLiteral("window.panels.color")},
        {tr("Split Toning"), QStringLiteral("window.panels.swatches")},
        {tr("Lens Corrections"), QStringLiteral("window.panels.patterns")},
        {tr("Effects"), QStringLiteral("layers.fx")},
        {tr("Camera Calibration"), QStringLiteral("tool.colorsampler")},
    };
    for (const Tab& entry : tabDefs) {
        const int index = tabs_->addTab(pictura::icon(entry.iconId), QString());
        tabs_->setTabToolTip(index, entry.title);
    }
    connect(tabs_, &QTabBar::currentChanged, stack_, &QStackedWidget::setCurrentIndex);

    buildControlPanels();

    right->addWidget(tabs_);
    right->addWidget(stack_, 1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &PicturaRawDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &PicturaRawDialog::reject);
    if (asShot) {
        auto* saveImage = new QPushButton(tr("Save Image…"), this);
        saveImage->setObjectName(QStringLiteral("rawSaveImageButton"));
        connect(saveImage, &QPushButton::clicked, this, &PicturaRawDialog::saveImage);
        saveStatus_ = new QLabel(this);
        saveStatus_->setObjectName(QStringLiteral("rawSaveStatus"));
        saveStatus_->setWordWrap(true);
        auto* saveRow = new QHBoxLayout;
        saveRow->addWidget(saveImage);
        saveRow->addWidget(saveStatus_, 1);
        right->addLayout(saveRow);
        auto* done = buttons->addButton(tr("Done"), QDialogButtonBox::ActionRole);
        done->setObjectName(QStringLiteral("rawDoneButton"));
        connect(done, &QPushButton::clicked, this, [this] {
            saveOnly_ = true;
            reject();
        });
    }
    right->addWidget(buttons);

    outer->addLayout(right);
    root->addLayout(outer, 1);

    auto* status = new QHBoxLayout;
    status->setContentsMargins(0, 0, 0, 0);
    status->setSpacing(8);
    photoName_ = new QLabel(this);
    photoName_->setObjectName(QStringLiteral("rawPhotoName"));
    photoName_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    photoInfo_ = new QLabel(this);
    photoInfo_->setObjectName(QStringLiteral("rawPhotoInfo"));
    photoInfo_->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    photoInfo_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    status->addWidget(photoName_);
    status->addStretch(1);
    status->addWidget(photoInfo_);
    auto* grip = new QSizeGrip(this);
    grip->setObjectName(QStringLiteral("rawSizeGrip"));
    status->addWidget(grip);
    root->addLayout(status);

    refreshTimer_ = new QTimer(this);
    refreshTimer_->setSingleShot(true);
    refreshTimer_->setInterval(30);
    connect(refreshTimer_, &QTimer::timeout, this, [this] {
        if (renderPending_) {
            renderPending_ = false;
            render();
            refreshTimer_->start();
        }
    });

    resize(1100, 680);
    render();
    updatePhotoInfo();
}

void PicturaRawDialog::updatePhotoInfo()
{
    QMap<QString, QString> info;
    if (view_) {
        const QStringList rows = pictura::photo_info(*view_);
        for (const QString& row : rows) {
            const int tab = row.indexOf(QLatin1Char('\t'));
            if (tab > 0) {
                info.insert(row.left(tab), row.mid(tab + 1));
            }
        }
    }

    const auto append = [](QString& text, const QString& value) {
        if (value.isEmpty()) {
            return;
        }
        if (!text.isEmpty()) {
            text += QStringLiteral("  ");
        }
        text += value;
    };
    QString top;
    QString bottom;
    append(top, info.value(QStringLiteral("f_number")));
    append(top, info.value(QStringLiteral("exposure")));
    append(bottom, info.value(QStringLiteral("iso")));
    append(bottom, info.value(QStringLiteral("focal_length")));
    histogram_->setExposureInfo(top, bottom);

    if (photoName_) {
        photoName_->setText(info.value(QStringLiteral("name")));
    }
    if (!photoInfo_) {
        return;
    }
    QStringList parts;
    const QString profile = info.value(QStringLiteral("profile"));
    if (!profile.isEmpty()) {
        parts.append(profile);
    }
    const QString depth = info.value(QStringLiteral("depth"));
    if (!depth.isEmpty()) {
        parts.append(tr("%1 bits").arg(depth));
    }
    const int width = info.value(QStringLiteral("width")).toInt();
    const int height = info.value(QStringLiteral("height")).toInt();
    if (width > 0 && height > 0) {
        const double megapixels = double(width) * double(height) / 1e6;
        parts.append(tr("%1 by %2 (%3MP)").arg(width).arg(height).arg(megapixels, 0, 'f', 1));
    }
    const QString ppi = info.value(QStringLiteral("ppi"));
    if (!ppi.isEmpty()) {
        parts.append(ppi);
    }
    const qint64 bytes = info.value(QStringLiteral("file_size")).toLongLong();
    if (bytes > 0) {
        parts.append(tr("%1 MB").arg(double(bytes) / (1024.0 * 1024.0), 0, 'f', 1));
    }
    photoInfo_->setText(parts.join(QStringLiteral("; ")));
}

void PicturaRawDialog::buildControlPanels()
{
    const auto add = [this](AcrPanel* panel) {
        if (auto* layout = panel->layout()) {
            layout->setContentsMargins(12, 2, 8, 2);
        }
        stack_->insertWidget(controlPanels_.size(), panel);
        connect(panel, &AcrPanel::changed, this, &PicturaRawDialog::refresh);
        controlPanels_.append(panel);
    };
    add(new AcrBasicPanel(&settings_, this, asShot_));
    auto* toneCurve = new AcrToneCurvePanel(&settings_, this);
    toneCurve_ = toneCurve;
    add(toneCurve);
    add(new AcrDetailPanel(&settings_, this));
    add(new AcrHslPanel(&settings_, this));
    add(new AcrSplitToningPanel(&settings_, this));
    add(new AcrLensPanel(&settings_, this));
    add(new AcrEffectsPanel(&settings_, this));
    add(new AcrCalibrationPanel(&settings_, this));
}

void PicturaRawDialog::refresh()
{
    // Throttle: render at once when idle, then at most every 30 ms, with a
    // trailing render so the last value of a drag is not dropped.
    if (!refreshTimer_) {
        render();
        return;
    }
    if (refreshTimer_->isActive()) {
        renderPending_ = true;
        return;
    }
    render();
    refreshTimer_->start();
}

void PicturaRawDialog::render()
{
    const QString text = settings_.encode();
    preview_->setSettingsText(text);
    histogram_->setImage(preview_->image());
    if (toneCurve_) {
        toneCurve_->setPreviewImage(preview_->image());
    }
}

void PicturaRawDialog::saveImage()
{
    const auto run = runner_ ? runner_ : [this](QDialog& dialog) {
        if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
            box->exec();
            return true;
        }
        return runDialog(dialog, this) == QDialog::Accepted;
    };
    const QString message = runRawSaveImage(this, view_, settings_.encode(), run);
    saveStatus_->setText(message);
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

#include "moc_pictura_raw_dialog.cpp"
