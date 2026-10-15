#include "reduce_noise_dialog.h"
#include "dialogs.h"
#include "filter_param_controls.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

namespace pictura {

namespace {

const int kPadSize = 250;

} // namespace

ReduceNoiseDialog::ReduceNoiseDialog(PictureView* view, const FilterCommandSpec& spec,
                                     QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec)
{
    setWindowTitle(spec.label);

    // Slots 0..4: Strength, Preserve Details, Reduce Color Noise, Sharpen
    // Details, Remove JPEG Artifacts.
    basic_ = new FilterParamControls(spec_.params.sliced(0, 5), this);
    // Slots 6..11: the per-channel pairs. Advanced edits one pair at a time, so
    // it gets a private two-row control whose labels drop the channel name.
    QList<FilterParamSpec> advParams;
    advParams.append(spec_.params.at(6));
    advParams.append(spec_.params.at(7));
    advParams[0].label = QStringLiteral("Strength:");
    advParams[1].label = QStringLiteral("Preserve Details:");
    advanced_ = new FilterParamControls(advParams, this);
    perChannel_ = QList<double>{
        spec_.params.at(6).value,  spec_.params.at(7).value,  spec_.params.at(8).value,
        spec_.params.at(9).value,  spec_.params.at(10).value, spec_.params.at(11).value};

    if (view_ && view_->has_document()) {
        const QImage image = documentPreviewImage(view_, kPadSize);
        if (!image.isNull()) {
            proxy_ = image.scaled(kPadSize, kPadSize, Qt::KeepAspectRatio, Qt::SmoothTransformation)
                         .convertToFormat(QImage::Format_RGBA8888);
            for (int y = 0; y < proxy_.height(); ++y) {
                proxyRgba_.append(reinterpret_cast<const char*>(proxy_.constScanLine(y)),
                                  proxy_.width() * 4);
            }
        }
    }

    pad_ = new QLabel(this);
    pad_->setObjectName(QStringLiteral("reduceNoisePad"));
    pad_->setFixedSize(kPadSize, kPadSize);
    pad_->setAlignment(Qt::AlignCenter);

    channel_ = new QComboBox(this);
    channel_->setObjectName(QStringLiteral("reduceNoiseChannel"));
    channel_->addItems({QStringLiteral("Red"), QStringLiteral("Green"), QStringLiteral("Blue")});

    tabs_ = new QTabWidget(this);
    tabs_->setObjectName(QStringLiteral("reduceNoiseTabs"));
    auto* basicPage = new QWidget(tabs_);
    auto* basicLayout = new QVBoxLayout(basicPage);
    for (QWidget* row : basic_->rows()) {
        basicLayout->addWidget(row);
    }
    basicLayout->addStretch(1);
    auto* advancedPage = new QWidget(tabs_);
    auto* advancedLayout = new QVBoxLayout(advancedPage);
    auto* channelRow = new QHBoxLayout;
    channelRow->addWidget(new QLabel(QStringLiteral("Channel:"), advancedPage));
    channelRow->addWidget(channel_);
    channelRow->addStretch(1);
    advancedLayout->addLayout(channelRow);
    for (QWidget* row : advanced_->rows()) {
        advancedLayout->addWidget(row);
    }
    advancedLayout->addStretch(1);
    tabs_->addTab(basicPage, QStringLiteral("Basic"));
    tabs_->addTab(advancedPage, QStringLiteral("Advanced"));

    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    preview_ = new QCheckBox(QStringLiteral("Preview"), this);
    preview_->setObjectName(QStringLiteral("filterPreview"));
    preview_->setChecked(true);
    connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
        if (on) {
            valuesChanged(true);
        } else {
            discardPreview();
        }
    });
    auto* buttons = new QVBoxLayout;
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addSpacing(6);
    buttons->addWidget(preview_);
    buttons->addStretch(1);

    auto* middle = new QVBoxLayout;
    middle->addWidget(pad_, 0, Qt::AlignHCenter);
    middle->addWidget(tabs_, 1);

    auto* outer = new QHBoxLayout(this);
    outer->addLayout(middle, 1);
    outer->addLayout(buttons, 0);
    outer->setSizeConstraint(QLayout::SetFixedSize);

    connect(basic_, &FilterParamControls::changed, this, [this] { valuesChanged(true); });
    connect(basic_, &FilterParamControls::changedLive, this, [this] { valuesChanged(false); });
    connect(advanced_, &FilterParamControls::changed, this, [this] {
        storeChannel();
        valuesChanged(true);
    });
    connect(advanced_, &FilterParamControls::changedLive, this, [this] {
        storeChannel();
        valuesChanged(false);
    });
    connect(channel_, &QComboBox::currentIndexChanged, this, [this](int) {
        reloadChannel();
        valuesChanged(true);
    });
    connect(tabs_, &QTabWidget::currentChanged, this, [this](int) { valuesChanged(true); });
    connect(this, &QDialog::rejected, this, &ReduceNoiseDialog::discardPreview);

    loadChannel();
}

int ReduceNoiseDialog::channelSlot() const
{
    return loadedChannel_ * 2;
}

void ReduceNoiseDialog::storeChannel()
{
    const int slot = channelSlot();
    const QList<double> v = advanced_->values();
    if (v.size() == 2) {
        perChannel_[slot] = v.at(0);
        perChannel_[slot + 1] = v.at(1);
    }
}

void ReduceNoiseDialog::loadChannel()
{
    const int slot = channelSlot();
    advanced_->setValues(QList<double>{perChannel_.at(slot), perChannel_.at(slot + 1)});
}

void ReduceNoiseDialog::reloadChannel()
{
    // The combo already moved, so store the sliders under the channel they
    // were loaded for before loading the newly selected one.
    storeChannel();
    loadedChannel_ = channel_->currentIndex();
    loadChannel();
}

QList<double> ReduceNoiseDialog::values() const
{
    QList<double> v;
    v += basic_->values(); // slots 0..4
    v.append(tabs_->currentIndex() == 1 ? 1.0 : 0.0); // slot 5: the Advanced flag
    v += perChannel_; // slots 6..11
    return v;
}

void ReduceNoiseDialog::setValues(const QList<double>& values)
{
    if (values.isEmpty()) {
        basic_->setValues(QList<double>());
        perChannel_ = QList<double>{
            spec_.params.at(6).value,  spec_.params.at(7).value,  spec_.params.at(8).value,
            spec_.params.at(9).value,  spec_.params.at(10).value, spec_.params.at(11).value};
    } else {
        if (values.size() >= 5) {
            basic_->setValues(values.sliced(0, 5));
        }
        if (values.size() >= 12) {
            perChannel_ = values.sliced(6, 6);
            if (values.at(5) >= 0.5) {
                QSignalBlocker block(tabs_);
                tabs_->setCurrentIndex(1);
            }
        }
    }
    loadedChannel_ = channel_->currentIndex();
    loadChannel();
    valuesChanged(false);
}

void ReduceNoiseDialog::renderPad()
{
    if (proxy_.isNull()) {
        return;
    }
    const QImage filtered =
        filter_thumbnail(proxyRgba_, proxy_.width(), proxy_.height(), spec_.kind, values());
    pad_->setPixmap(QPixmap::fromImage(filtered.isNull() ? proxy_ : filtered));
}

void ReduceNoiseDialog::valuesChanged(bool canvas)
{
    renderPad();
    if (!canvas || !shownOnce_ || !preview_->isChecked() || !view_ || !view_->has_document()) {
        return;
    }
    if (filter_preview(*view_, spec_.kind, values())) {
        previewShown_ = true;
    }
}

void ReduceNoiseDialog::discardPreview()
{
    if (previewShown_ && view_) {
        filter_preview_cancel(*view_);
        previewShown_ = false;
    }
}

void ReduceNoiseDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // Preview on open, deferred a turn so the dialog paints before the render.
    if (!shownOnce_) {
        shownOnce_ = true;
        QTimer::singleShot(0, this, [this] {
            if (isVisible()) {
                valuesChanged(true);
            }
        });
    }
}

bool ReduceNoiseDialog::get(PictureView* view, const FilterCommandSpec& spec,
                            const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    ReduceNoiseDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura
