#include "picture_frame_dialog.h"
#include "dialogs.h"
#include "filter_param_controls.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QIcon>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

namespace pictura {

namespace {

const int kPadSize = 250;
const int kIconSize = 48;

// The spec's Basic controls, in order. The frame is the picker's slot; the
// rest follow the mapping order.
enum Basic { kVine, kMargin, kSize, kArrangement, kFlower, kFlowerColour, kFlowerSize, kLeaf,
             kLeafColour, kLeafSize };
// The spec's Advanced controls, in order.
enum Advanced { kLines, kThickness, kAngle, kFade, kInvert };

// A frame's slot list: the given values with the frame slot replaced, so the
// picker thumbnails and the pad share the mapping.
QList<double> withFrame(const QList<double>& values, int frame)
{
    QList<double> v = values;
    if (v.isEmpty()) {
        v = QList<double>(22, 0.0);
    }
    v[0] = frame;
    return v;
}

} // namespace

PictureFrameDialog::PictureFrameDialog(PictureView* view, const FilterCommandSpec& spec,
                                       QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec)
{
    setWindowTitle(spec.label);

    // The Basic page: the picker owns the frame slot, so the spec's Frame
    // control becomes a Hidden slot here.
    QList<FilterParamSpec> basicParams = spec_.params.sliced(0, 11);
    basicParams[0].control = FilterControl::Hidden;
    const QList<FilterParamSpec> advancedParams = spec_.params.sliced(11, 5);

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

    picker_ = new QListWidget(this);
    picker_->setObjectName(QStringLiteral("pictureFramePicker"));
    picker_->setViewMode(QListView::IconMode);
    picker_->setIconSize(QSize(kIconSize, kIconSize));
    picker_->setGridSize(QSize(kIconSize + 24, kIconSize + 24));
    picker_->setResizeMode(QListView::Adjust);
    picker_->setMovement(QListView::Static);
    picker_->setUniformItemSizes(true);
    picker_->setMinimumWidth(kIconSize * 2 + 48);

    pad_ = new QLabel(this);
    pad_->setObjectName(QStringLiteral("pictureFramePad"));
    pad_->setFixedSize(kPadSize, kPadSize);
    pad_->setAlignment(Qt::AlignCenter);

    basic_ = new FilterParamControls(basicParams, this, proxy_);
    advanced_ = new FilterParamControls(advancedParams, this);
    auto* tabs = new QTabWidget(this);
    tabs->setObjectName(QStringLiteral("pictureFrameTabs"));
    auto* basicPage = new QWidget(tabs);
    auto* basicLayout = new QVBoxLayout(basicPage);
    for (QWidget* row : basic_->rows()) {
        basicLayout->addWidget(row);
    }
    basicLayout->addStretch(1);
    auto* advancedPage = new QWidget(tabs);
    auto* advancedLayout = new QVBoxLayout(advancedPage);
    for (QWidget* row : advanced_->rows()) {
        advancedLayout->addWidget(row);
    }
    advancedLayout->addStretch(1);
    tabs->addTab(basicPage, QStringLiteral("Basic"));
    tabs->addTab(advancedPage, QStringLiteral("Advanced"));

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
    middle->addWidget(tabs, 1);

    auto* outer = new QHBoxLayout(this);
    outer->addWidget(picker_, 0);
    outer->addLayout(middle, 1);
    outer->addLayout(buttons, 0);
    outer->setSizeConstraint(QLayout::SetFixedSize);

    connect(picker_, &QListWidget::currentRowChanged, this, [this](int row) {
        if (row >= 0) {
            setFrame(row);
        }
    });
    connect(basic_, &FilterParamControls::changed, this, [this] { valuesChanged(true); });
    connect(basic_, &FilterParamControls::changedLive, this, [this] { valuesChanged(false); });
    connect(advanced_, &FilterParamControls::changed, this, [this] { valuesChanged(true); });
    connect(advanced_, &FilterParamControls::changedLive, this, [this] { valuesChanged(false); });
    connect(this, &QDialog::rejected, this, &PictureFrameDialog::discardPreview);

    buildPicker();
    setValues(QList<double>());
}

void PictureFrameDialog::buildPicker()
{
    const QList<double> defaults = values();
    // A small white ground for the thumbnails, so a frame reads the same as it
    // does on a white picture.
    QImage ground(kIconSize, kIconSize, QImage::Format_RGBA8888);
    ground.fill(QColor(255, 255, 255));
    QByteArray groundRgba;
    for (int y = 0; y < ground.height(); ++y) {
        groundRgba.append(reinterpret_cast<const char*>(ground.constScanLine(y)),
                          ground.width() * 4);
    }
    QSignalBlocker block(picker_);
    for (int i = 0; i < spec_.params.at(0).choices.size(); ++i) {
        const QImage thumb = filter_thumbnail(groundRgba, kIconSize, kIconSize, spec_.kind,
                                              withFrame(defaults, i));
        auto* item = new QListWidgetItem(QIcon(QPixmap::fromImage(thumb)),
                                         spec_.params.at(0).choices.at(i), picker_);
        item->setToolTip(spec_.params.at(0).choices.at(i));
    }
}

QList<double> PictureFrameDialog::values() const
{
    QList<double> v = basic_->values();
    v += advanced_->values();
    return v;
}

void PictureFrameDialog::setValues(const QList<double>& values)
{
    if (values.isEmpty()) {
        basic_->setValues(QList<double>());
        advanced_->setValues(QList<double>());
    } else {
        if (values.size() >= 17) {
            basic_->setValues(values.sliced(0, 17));
        }
        if (values.size() >= 22) {
            advanced_->setValues(values.sliced(17));
        }
    }
    const int frame = values.isEmpty() ? static_cast<int>(spec_.params.at(0).value)
                                       : static_cast<int>(values.at(0));
    {
        QSignalBlocker block(picker_);
        picker_->setCurrentRow(frame);
    }
    if (QWidget* lines = advanced_->rows().value(kLines)) {
        lines->setEnabled(filter_picture_frame_uses_lines(frame + 1));
    }
    valuesChanged(false);
}

void PictureFrameDialog::setFrame(int frame)
{
    QList<double> v = basic_->values();
    v[0] = frame;
    basic_->setValues(v);
    if (QWidget* lines = advanced_->rows().value(kLines)) {
        lines->setEnabled(filter_picture_frame_uses_lines(frame + 1));
    }
    valuesChanged(true);
}

QImage PictureFrameDialog::padImage() const
{
    return pad_->pixmap().toImage();
}

void PictureFrameDialog::renderPad()
{
    if (proxy_.isNull()) {
        return;
    }
    const QImage framed =
        filter_thumbnail(proxyRgba_, proxy_.width(), proxy_.height(), spec_.kind, values());
    pad_->setPixmap(QPixmap::fromImage(framed.isNull() ? proxy_ : framed));
}

void PictureFrameDialog::valuesChanged(bool canvas)
{
    renderPad();
    if (!canvas || !shownOnce_ || !preview_->isChecked() || !view_ || !view_->has_document()) {
        return;
    }
    if (filter_preview(*view_, spec_.kind, values())) {
        previewShown_ = true;
    }
}

void PictureFrameDialog::discardPreview()
{
    if (previewShown_ && view_) {
        filter_preview_cancel(*view_);
        previewShown_ = false;
    }
}

void PictureFrameDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // Preview on open, deferred a turn so the dialog paints before the
    // whole-layer render runs.
    if (!shownOnce_) {
        shownOnce_ = true;
        QTimer::singleShot(0, this, [this] {
            if (isVisible()) {
                valuesChanged(true);
            }
        });
    }
}

bool PictureFrameDialog::get(PictureView* view, const FilterCommandSpec& spec,
                             const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    PictureFrameDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura

#include "moc_picture_frame_dialog.cpp"
