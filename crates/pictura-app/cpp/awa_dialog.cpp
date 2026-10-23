#include "awa_dialog.h"
#include "awa_canvas.h"
#include "dialogs.h"
#include "icons.h"
#include "panels/jump_slider.h"

#include <QtCore/QDebug>
#include <QtCore/QEvent>
#include <QtCore/QSignalBlocker>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeySequence>
#include <QtGui/QScreen>
#include <QtGui/QShortcut>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <vector>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/adaptive_wide_angle.cxxqt.h"

namespace pictura {

namespace {

const QString kHelpIdle = QStringLiteral("Place mouse over a control for help");

struct ToolInfo {
    AdaptiveWideAngleDialog::Tool tool;
    const char* icon;
    const char* tip;
    Qt::Key key;
};

const ToolInfo kTools[] = {
    {AdaptiveWideAngleDialog::Constraint, "tool.line",
     "Constraint Tool: click on the image or drag an end point to add or edit a constraint. "
     "Shift-drag for a horizontal or vertical constraint.",
     Qt::Key_C},
    {AdaptiveWideAngleDialog::Polygon, "tool.polygon",
     "Polygon Constraint Tool: click on the image to add corners; click the first corner to "
     "close the polygon.",
     Qt::Key_Y},
    {AdaptiveWideAngleDialog::Move, "tool.move", "Move Tool: drag to move the content in the canvas.",
     Qt::Key_M},
    {AdaptiveWideAngleDialog::Hand, "tool.hand", "Hand Tool: drag to move the view.", Qt::Key_H},
    {AdaptiveWideAngleDialog::Zoom, "tool.zoom",
     "Zoom Tool: click to zoom in; Alt-click to zoom out.", Qt::Key_Z},
};

// A label over a slider, its value box and unit on the right, as CS6 lays
// the Correction controls out. The slider runs in hundredths for a double.
QWidget* sliderRow(QWidget* parent, const QString& label, QAbstractSpinBox* box, double minimum,
                   double maximum, double scale, const QString& unit)
{
    auto* row = new QWidget(parent);
    auto* layout = new QVBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(1);
    auto* top = new QHBoxLayout;
    auto* caption = new QLabel(label, row);
    caption->setBuddy(box);
    top->addWidget(caption);
    top->addStretch(1);
    top->addWidget(box);
    top->addWidget(new QLabel(unit, row));
    auto* slider = new JumpSlider(Qt::Horizontal, row);
    slider->setObjectName(box->objectName() + QStringLiteral("Slider"));
    slider->setRange(static_cast<int>(std::lround(minimum * scale)),
                     static_cast<int>(std::lround(maximum * scale)));
    layout->addLayout(top);
    layout->addWidget(slider);
    auto sync = [slider, scale](double v) {
        QSignalBlocker block(slider);
        slider->setValue(static_cast<int>(std::lround(v * scale)));
    };
    if (auto* d = qobject_cast<QDoubleSpinBox*>(box)) {
        QObject::connect(d, &QDoubleSpinBox::valueChanged, slider, sync);
        QObject::connect(slider, &QSlider::valueChanged, d,
                         [d, scale](int v) { d->setValue(v / scale); });
        sync(d->value());
    } else if (auto* s = qobject_cast<QSpinBox*>(box)) {
        QObject::connect(s, &QSpinBox::valueChanged, slider, sync);
        QObject::connect(slider, &QSlider::valueChanged, s, &QSpinBox::setValue);
        sync(s->value());
    }
    return row;
}

} // namespace

AdaptiveWideAngleDialog::AdaptiveWideAngleDialog(PictureView* view, const QString& documentName,
                                                 QWidget* parent)
    : QDialog(parent), view_(view), documentName_(documentName)
{
    setObjectName(QStringLiteral("awaDialog"));
    ready_ = view_ && awa_begin(*view_);

    // The tool column.
    tools_ = new QButtonGroup(this);
    tools_->setExclusive(true);
    auto* column = new QVBoxLayout;
    column->setSpacing(2);
    for (const ToolInfo& info : kTools) {
        if (info.tool == Hand) {
            column->addSpacing(6); // CS6 rules the column before Hand
        }
        auto* button = new QToolButton(this);
        button->setObjectName(QStringLiteral("awaTool%1").arg(int(info.tool)));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setIcon(icon(QString::fromLatin1(info.icon)));
        button->setIconSize(QSize(20, 20));
        button->setToolTip(QStringLiteral("%1 (%2)").arg(QString::fromLatin1(info.tip),
                                                         QKeySequence(info.key).toString()));
        button->installEventFilter(this);
        tools_->addButton(button, info.tool);
        column->addWidget(button);
        if (info.tool == Zoom) {
            button->setContextMenuPolicy(Qt::CustomContextMenu);
            connect(button, &QToolButton::customContextMenuRequested, this,
                    [this, button](const QPoint& at) {
                        canvas_->showZoomMenu(button->mapToGlobal(at));
                    });
        }
        auto* shortcut = new QShortcut(QKeySequence(info.key), this);
        connect(shortcut, &QShortcut::activated, this, [this, tool = info.tool] { selectTool(tool); });
    }
    column->addStretch(1);
    connect(tools_, &QButtonGroup::idToggled, this, [this](int id, bool on) {
        if (on) {
            canvas_->setMode(static_cast<AwaCanvas::Mode>(id));
        }
    });

    // The help line, the preview, its zoom row and the lens lines.
    help_ = new QLabel(kHelpIdle, this);
    help_->setObjectName(QStringLiteral("awaHelp"));
    canvas_ = new AwaCanvas(this);
    auto* zoomOut = new QToolButton(this);
    zoomOut->setText(QStringLiteral("−"));
    auto* zoomIn = new QToolButton(this);
    zoomIn->setText(QStringLiteral("+"));
    zoomLabel_ = new QLabel(this);
    zoomLabel_->setObjectName(QStringLiteral("awaZoom"));
    zoomLabel_->setMinimumWidth(56);
    zoomLabel_->setAlignment(Qt::AlignCenter);
    connect(zoomOut, &QToolButton::clicked, this, [this] { canvas_->zoomBy(0.5); });
    connect(zoomIn, &QToolButton::clicked, this, [this] { canvas_->zoomBy(2.0); });
    addZoomShortcuts(this, zoomIn, zoomOut);
    auto orDashes = [](const QString& text) {
        return text.isEmpty() ? QStringLiteral("--") : text;
    };
    auto* camera = new QLabel(
        QStringLiteral("Camera Model: %1").arg(orDashes(ready_ ? QString(awa_camera_model()) : QString())),
        this);
    camera->setObjectName(QStringLiteral("awaCamera"));
    auto* lens = new QLabel(
        QStringLiteral("Lens Model: %1").arg(orDashes(ready_ ? QString(awa_lens_model()) : QString())),
        this);
    lens->setObjectName(QStringLiteral("awaLens"));
    preview_ = new QCheckBox(QStringLiteral("Preview"), this);
    preview_->setObjectName(QStringLiteral("awaPreview"));
    preview_->setChecked(true);
    showConstraints_ = new QCheckBox(QStringLiteral("Show Constraints"), this);
    showConstraints_->setObjectName(QStringLiteral("awaShowConstraints"));
    showConstraints_->setChecked(true);
    showMesh_ = new QCheckBox(QStringLiteral("Show Mesh"), this);
    showMesh_->setObjectName(QStringLiteral("awaShowMesh"));
    connect(preview_, &QCheckBox::toggled, canvas_, &AwaCanvas::setPreview);
    connect(showConstraints_, &QCheckBox::toggled, canvas_, &AwaCanvas::setShowConstraints);
    connect(showMesh_, &QCheckBox::toggled, canvas_, &AwaCanvas::setShowMesh);
    const struct {
        QCheckBox* box;
        Qt::Key key;
    } toggles[] = {{preview_, Qt::Key_P}, {showConstraints_, Qt::Key_W}, {showMesh_, Qt::Key_E}};
    for (const auto& t : toggles) {
        auto* shortcut = new QShortcut(QKeySequence(Qt::CTRL | t.key), this);
        connect(shortcut, &QShortcut::activated, t.box, &QCheckBox::toggle);
    }
    auto* bottom = new QHBoxLayout;
    bottom->addWidget(zoomOut);
    bottom->addWidget(zoomLabel_);
    bottom->addWidget(zoomIn);
    auto* lensLines = new QVBoxLayout;
    lensLines->setSpacing(0);
    lensLines->addWidget(camera);
    lensLines->addWidget(lens);
    bottom->addLayout(lensLines);
    bottom->addStretch(1);
    bottom->addWidget(preview_);
    bottom->addWidget(showConstraints_);
    bottom->addWidget(showMesh_);
    auto* centre = new QVBoxLayout;
    centre->addWidget(help_);
    centre->addWidget(canvas_, 1);
    centre->addLayout(bottom);

    // The options column.
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);

    auto* lensBox = new QGroupBox(this);
    auto* lensLayout = new QVBoxLayout(lensBox);
    correction_ = new QComboBox(lensBox);
    correction_->setObjectName(QStringLiteral("awaCorrection"));
    correction_->addItems(
        {QStringLiteral("Fisheye"), QStringLiteral("Perspective"), QStringLiteral("Full Spherical")});
    correction_->setToolTip(QStringLiteral("Correction: the projection the image was taken with."));
    if (auto* model = qobject_cast<QStandardItemModel*>(correction_->model())) {
        // Full Spherical needs a 2:1 panorama.
        model->item(2)->setEnabled(ready_ && awa_full_spherical_allowed());
    }
    auto* correctionRow = new QHBoxLayout;
    correctionRow->addWidget(new QLabel(QStringLiteral("Correction:"), lensBox));
    correctionRow->addWidget(correction_, 1);
    lensLayout->addLayout(correctionRow);

    scale_ = new QSpinBox(lensBox);
    scale_->setObjectName(QStringLiteral("awaScale"));
    scale_->setRange(50, 150);
    scale_->setValue(100);
    scale_->setToolTip(QStringLiteral("Scale: scales the image to reduce blank areas."));
    focal_ = new QDoubleSpinBox(lensBox);
    focal_->setObjectName(QStringLiteral("awaFocal"));
    focal_->setDecimals(2);
    focal_->setToolTip(QStringLiteral("Focal Length: the lens's focal length."));
    crop_ = new QDoubleSpinBox(lensBox);
    crop_->setObjectName(QStringLiteral("awaCrop"));
    crop_->setDecimals(2);
    crop_->setRange(0.1, 10.0);
    crop_->setToolTip(QStringLiteral("Crop Factor: the sensor's crop factor."));
    lensLayout->addWidget(sliderRow(lensBox, QStringLiteral("Scale"), scale_, 50, 150, 1.0,
                                    QStringLiteral("%")));
    // The session opens with the EXIF lens or the default; Focal Length's
    // range follows Crop Factor.
    const ::rust::Vec<double> opening = ready_ ? awa_lens_settings() : ::rust::Vec<double>();
    if (opening.size() == 2) {
        crop_->setValue(opening[1]);
    }
    lensLayout->addWidget(sliderRow(lensBox, QStringLiteral("Focal Length"), focal_, 0.0, 1.0,
                                    100.0, QStringLiteral("mm")));
    updateFocalRange();
    if (opening.size() == 2) {
        focal_->setValue(opening[0]);
    }
    lensLayout->addWidget(
        sliderRow(lensBox, QStringLiteral("Crop Factor"), crop_, 0.1, 10.0, 100.0, QString()));
    asShot_ = new QCheckBox(QStringLiteral("As Shot"), lensBox);
    asShot_->setObjectName(QStringLiteral("awaAsShot"));
    asShot_->setToolTip(QStringLiteral("As Shot: use the lens information the image carries."));
    lensLayout->addWidget(asShot_);

    if (ready_) {
        for (double v : awa_exif_lens()) {
            exifLens_.append(v);
        }
    }
    asShot_->setEnabled(exifLens_.size() == 2);
    asShot_->setChecked(exifLens_.size() == 2);
    connect(asShot_, &QCheckBox::toggled, this, &AdaptiveWideAngleDialog::asShotToggled);
    connect(correction_, &QComboBox::currentIndexChanged, this,
            &AdaptiveWideAngleDialog::correctionChanged);
    connect(scale_, &QSpinBox::valueChanged, this, &AdaptiveWideAngleDialog::pushLens);
    connect(crop_, &QDoubleSpinBox::valueChanged, this, &AdaptiveWideAngleDialog::updateFocalRange);
    for (QDoubleSpinBox* box : {focal_, crop_}) {
        connect(box, &QDoubleSpinBox::valueChanged, this, [this] {
            if (asShot_->isChecked() && exifLens_.size() == 2 &&
                (focal_->value() != exifLens_.at(0) || crop_->value() != exifLens_.at(1))) {
                QSignalBlocker block(asShot_);
                asShot_->setChecked(false);
            }
            pushLens();
        });
    }
    for (QWidget* w : QList<QWidget*>{correction_, scale_, focal_, crop_, asShot_}) {
        w->installEventFilter(this);
    }

    auto* detailBox = new QGroupBox(QStringLiteral("Detail"), this);
    auto* detailLayout = new QVBoxLayout(detailBox);
    detail_ = new AwaDetail(detailBox);
    detailLayout->addWidget(detail_);
    canvas_->hoverMoved = [this](const QPointF& display) {
        detail_->showAt(display, canvas_->preview());
    };

    auto* side = new QVBoxLayout;
    side->addWidget(ok);
    side->addWidget(cancel);
    side->addSpacing(10);
    side->addWidget(lensBox);
    side->addWidget(detailBox);
    side->addStretch(1);
    auto* sideWidget = new QWidget(this);
    sideWidget->setLayout(side);
    sideWidget->setFixedWidth(290);

    auto* outer = new QHBoxLayout(this);
    outer->addLayout(column);
    outer->addLayout(centre, 1);
    outer->addWidget(sideWidget);

    canvas_->zoomChanged = [this] { updateZoomLabel(); };
    selectTool(Constraint);
    if (const QScreen* screen = parent ? parent->screen() : QGuiApplication::primaryScreen()) {
        resize(screen->availableGeometry().size() * 0.85);
    }
    if (ready_) {
        correctionChanged();
    }
    canvas_->reload();
    updateZoomLabel();
}

AdaptiveWideAngleDialog::~AdaptiveWideAngleDialog()
{
    finish(false);
}

bool AdaptiveWideAngleDialog::load(const QList<double>& values)
{
    if (!ready_ || values.size() != 7) {
        return false;
    }
    {
        const QSignalBlocker a(correction_);
        const QSignalBlocker b(scale_);
        const QSignalBlocker c(focal_);
        const QSignalBlocker d(crop_);
        const QSignalBlocker e(asShot_);
        correction_->setCurrentIndex(std::clamp(static_cast<int>(std::lround(values.at(0))), 0, 2));
        scale_->setValue(static_cast<int>(std::lround(values.at(1))));
        crop_->setValue(values.at(3));
        updateFocalRange();
        focal_->setValue(values.at(2));
        asShot_->setChecked(exifLens_.size() == 2 && focal_->value() == exifLens_.at(0) &&
                            crop_->value() == exifLens_.at(1));
    }
    const std::vector<double> flat(values.begin(), values.end());
    if (!awa_load(::rust::Slice<const double>(flat.data(), flat.size()))) {
        return false;
    }
    correctionChanged();
    return true;
}

int AdaptiveWideAngleDialog::tool() const
{
    return tools_->checkedId();
}

void AdaptiveWideAngleDialog::selectTool(int tool)
{
    if (QAbstractButton* button = tools_->button(tool)) {
        button->setChecked(true);
    }
}

bool AdaptiveWideAngleDialog::eventFilter(QObject* watched, QEvent* event)
{
    if (auto* widget = qobject_cast<QWidget*>(watched)) {
        if (event->type() == QEvent::Enter) {
            help_->setText(widget->toolTip());
        } else if (event->type() == QEvent::Leave) {
            help_->setText(kHelpIdle);
        }
    }
    return QDialog::eventFilter(watched, event);
}

void AdaptiveWideAngleDialog::pushLens()
{
    if (!ready_) {
        return;
    }
    awa_set_lens(correction_->currentIndex(), scale_->value(), focal_->value(), crop_->value());
    canvas_->refreshAll();
}

void AdaptiveWideAngleDialog::updateFocalRange()
{
    const ::rust::Vec<double> range = awa_focal_range(crop_->value());
    if (range.size() != 2) {
        return;
    }
    {
        // A value past the new range is clamped; the crop edit pushes it.
        const QSignalBlocker block(focal_);
        focal_->setRange(range[0], range[1]);
    }
    if (auto* slider = findChild<QSlider*>(QStringLiteral("awaFocalSlider"))) {
        const QSignalBlocker block(slider);
        slider->setRange(static_cast<int>(std::lround(range[0] * 100.0)),
                         static_cast<int>(std::lround(range[1] * 100.0)));
        slider->setValue(static_cast<int>(std::lround(focal_->value() * 100.0)));
    }
}

void AdaptiveWideAngleDialog::asShotToggled(bool on)
{
    if (on && exifLens_.size() == 2) {
        const QSignalBlocker a(focal_);
        const QSignalBlocker b(crop_);
        crop_->setValue(exifLens_.at(1));
        updateFocalRange();
        focal_->setValue(exifLens_.at(0));
        pushLens();
    }
}

void AdaptiveWideAngleDialog::correctionChanged()
{
    // Full Spherical maps by longitude and latitude, not by a lens.
    const bool spherical = correction_->currentIndex() == 2;
    focal_->setEnabled(!spherical);
    crop_->setEnabled(!spherical);
    asShot_->setEnabled(!spherical && exifLens_.size() == 2);
    pushLens();
}

void AdaptiveWideAngleDialog::updateZoomLabel()
{
    const double percent = canvas_->zoom() * 100.0;
    const QString text = percent >= 10.0 ? QString::number(std::lround(percent))
                                         : QString::number(percent, 'f', 2);
    zoomLabel_->setText(text + QLatin1Char('%'));
    setWindowTitle(QStringLiteral("Adaptive Wide Angle (%1 @ %2%)").arg(documentName_, text));
}

void AdaptiveWideAngleDialog::finish(bool keep)
{
    if (ready_ && !finished_) {
        committed_.clear();
        for (double v : awa_finish(keep)) {
            committed_.append(v);
        }
        finished_ = true;
    }
}

void AdaptiveWideAngleDialog::accept()
{
    finish(true);
    QDialog::accept();
}

void AdaptiveWideAngleDialog::reject()
{
    finish(false);
    QDialog::reject();
}

bool AdaptiveWideAngleDialog::get(PictureView* view, const QString& documentName,
                                  const QList<double>& initial, QList<double>* out,
                                  QWidget* parent)
{
    AdaptiveWideAngleDialog dialog(view, documentName, parent);
    if (!dialog.ready()) {
        return false;
    }
    // Settings that no longer apply (a constraint set no longer kept, Full
    // Spherical on a layer that is not 2:1) refuse rather than open on
    // defaults OK would then commit.
    if (!initial.isEmpty() && !dialog.load(initial)) {
        qWarning().noquote() << "Adaptive Wide Angle: the last settings no longer apply";
        return false;
    }
    if (runDialog(dialog, parent) != QDialog::Accepted || dialog.committed().size() != 7) {
        return false;
    }
    *out = dialog.committed();
    return true;
}

} // namespace pictura

#include "moc_awa_dialog.cpp"
