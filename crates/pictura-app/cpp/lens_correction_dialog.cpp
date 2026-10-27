#include "lens_correction_dialog.h"
#include "color_picker_dialog.h"
#include "dialogs.h"
#include "icons.h"
#include "lens_correction_canvas.h"
#include "lens_correction_panels.h"
#include "panels/color_swatch_widget.h"

#include <QtCore/QDebug>
#include <QtCore/QEvent>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeySequence>
#include <QtGui/QScreen>
#include <QtGui/QShortcut>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <vector>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/lens_correction.cxxqt.h"

namespace pictura {

namespace {

constexpr int kSlots =
    LensAutoPanel::kSlots + LensCustomPanel::kSlots + LensAutoPanel::kProfileSlots;

struct ToolInfo {
    LensCorrectionDialog::Tool tool;
    const char* icon;
    const char* name;
    const char* hint;
    Qt::Key key;
};

const ToolInfo kTools[] = {
    {LensCorrectionDialog::RemoveDistortion, "options.warp", "Remove Distortion Tool",
     "Drag towards or away from center to correct distortion.", Qt::Key_D},
    {LensCorrectionDialog::Straighten, "crop.straighten", "Straighten Tool",
     "Draw a line to straighten the image to a new horizontal or vertical axis.", Qt::Key_A},
    {LensCorrectionDialog::MoveGrid, "crop.overlay.grid", "Move Grid Tool",
     "Drag to move the alignment grid.", Qt::Key_M},
    {LensCorrectionDialog::Hand, "tool.hand", "Hand Tool", "Drag to move the image in the window.",
     Qt::Key_H},
    {LensCorrectionDialog::Zoom, "tool.zoom", "Zoom Tool",
     "Click to zoom in; Alt-click to zoom out.", Qt::Key_Z},
};

QString orDashes(const QString& text)
{
    return text.isEmpty() ? QStringLiteral("--") : text;
}

} // namespace

LensCorrectionDialog::LensCorrectionDialog(PictureView* view, QWidget* parent)
    : QDialog(parent), view_(view)
{
    setObjectName(QStringLiteral("lcDialog"));
    ready_ = view_ && lc_begin(*view_);

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
        button->setObjectName(QStringLiteral("lcTool%1").arg(int(info.tool)));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setIcon(icon(QString::fromLatin1(info.icon)));
        button->setIconSize(QSize(20, 20));
        button->setToolTip(QStringLiteral("%1 (%2)").arg(QString::fromLatin1(info.name),
                                                         QKeySequence(info.key).toString()));
        button->installEventFilter(this);
        tools_->addButton(button, info.tool);
        column->addWidget(button);
        auto* shortcut = new QShortcut(QKeySequence(info.key), this);
        connect(shortcut, &QShortcut::activated, this, [this, tool = info.tool] { selectTool(tool); });
    }
    column->addStretch(1);
    connect(tools_, &QButtonGroup::idToggled, this, [this](int id, bool on) {
        if (on) {
            canvas_->setMode(static_cast<LensCorrectionCanvas::Mode>(id));
            showToolHelp();
        }
    });

    // The help line, the preview, and the row under it.
    help_ = new QLabel(this);
    help_->setObjectName(QStringLiteral("lcHelp"));
    canvas_ = new LensCorrectionCanvas(this);
    auto* zoomOut = new QToolButton(this);
    zoomOut->setText(QStringLiteral("−"));
    auto* zoomIn = new QToolButton(this);
    zoomIn->setText(QStringLiteral("+"));
    zoomLabel_ = new QLabel(this);
    zoomLabel_->setObjectName(QStringLiteral("lcZoom"));
    zoomLabel_->setMinimumWidth(56);
    zoomLabel_->setAlignment(Qt::AlignCenter);
    connect(zoomOut, &QToolButton::clicked, this, [this] { canvas_->zoomBy(0.5); });
    connect(zoomIn, &QToolButton::clicked, this, [this] { canvas_->zoomBy(2.0); });
    addZoomShortcuts(this, zoomIn, zoomOut);

    QStringList exif;
    if (ready_) {
        for (const QString& s : lc_exif()) {
            exif.append(s);
        }
    }
    const QString camera = (exif.value(0) + QLatin1Char(' ') + exif.value(1)).trimmed();
    auto* cameraLine = new QLabel(QStringLiteral("Camera Model: %1").arg(orDashes(camera)), this);
    cameraLine->setObjectName(QStringLiteral("lcCamera"));
    auto* lensLine = new QLabel(QStringLiteral("Lens Model: %1").arg(orDashes(exif.value(2))), this);
    lensLine->setObjectName(QStringLiteral("lcLens"));
    auto* settingsLine = new QLabel(
        QStringLiteral("Camera Settings: %1")
            .arg(exif.value(3).isEmpty() ? QStringLiteral("--mm, f/--, --m") : exif.value(3)),
        this);
    settingsLine->setObjectName(QStringLiteral("lcCameraSettings"));

    preview_ = new QCheckBox(QStringLiteral("Preview"), this);
    preview_->setObjectName(QStringLiteral("lcPreview"));
    preview_->setChecked(true);
    connect(preview_, &QCheckBox::toggled, canvas_, &LensCorrectionCanvas::setPreview);
    showGrid_ = new QCheckBox(QStringLiteral("Show Grid"), this);
    showGrid_->setObjectName(QStringLiteral("lcShowGrid"));
    connect(showGrid_, &QCheckBox::toggled, canvas_, &LensCorrectionCanvas::setShowGrid);
    gridColor_ = new ColorSwatchWidget(this);
    gridColor_->setObjectName(QStringLiteral("lcGridColor"));
    gridColor_->setColor(canvas_->gridColor());
    gridColor_->setFixedSize(24, 16);
    gridColor_->setToolTip(QStringLiteral("Color: the grid's color."));
    connect(gridColor_, &QPushButton::clicked, this, &LensCorrectionDialog::pickGridColor);
    gridSize_ = new QSpinBox(this);
    gridSize_->setObjectName(QStringLiteral("lcGridSize"));
    gridSize_->setRange(4, 1024);
    gridSize_->setValue(canvas_->gridSize());
    gridSize_->setToolTip(QStringLiteral("Size: the grid's spacing, in pixels."));
    connect(gridSize_, &QSpinBox::valueChanged, canvas_, &LensCorrectionCanvas::setGridSize);
    for (QWidget* w : QList<QWidget*>{preview_, showGrid_, gridColor_, gridSize_}) {
        w->installEventFilter(this);
    }

    auto* bottom = new QHBoxLayout;
    bottom->addWidget(zoomOut);
    bottom->addWidget(zoomLabel_);
    bottom->addWidget(zoomIn);
    auto* lines = new QVBoxLayout;
    lines->setSpacing(0);
    lines->addWidget(cameraLine);
    lines->addWidget(lensLine);
    lines->addWidget(settingsLine);
    bottom->addLayout(lines);
    bottom->addStretch(1);
    bottom->addWidget(preview_);
    bottom->addWidget(showGrid_);
    bottom->addWidget(new QLabel(QStringLiteral("Color:"), this));
    bottom->addWidget(gridColor_);
    bottom->addWidget(new QLabel(QStringLiteral("Size:"), this));
    bottom->addWidget(gridSize_);
    auto* centre = new QVBoxLayout;
    centre->addWidget(help_);
    centre->addWidget(canvas_, 1);
    centre->addLayout(bottom);

    // OK / Cancel over the tabs.
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    auto_ = new LensAutoPanel(ready_, this);
    custom_ = new LensCustomPanel(ready_ && lc_grayscale(), this);
    auto* customScroll = new QScrollArea(this);
    customScroll->setWidgetResizable(true);
    customScroll->setFrameShape(QFrame::NoFrame);
    customScroll->setWidget(custom_);
    tabs_ = new QTabWidget(this);
    tabs_->setObjectName(QStringLiteral("lcTabs"));
    tabs_->addTab(auto_, QStringLiteral("Auto Correction"));
    tabs_->addTab(customScroll, QStringLiteral("Custom"));
    for (QWidget* panel : QList<QWidget*>{auto_, custom_}) {
        for (QWidget* control : panel->findChildren<QWidget*>()) {
            if (!control->toolTip().isEmpty()) {
                control->installEventFilter(this);
            }
        }
    }
    auto_->changed = [this] { pushValues(); };
    custom_->changed = [this] { pushValues(); };

    canvas_->distortionValue = [this] { return custom_->distortion(); };
    canvas_->distortionDragged = [this](double v) { custom_->setDistortion(v); };
    canvas_->straightened = [this](const QPointF& a, const QPointF& b) {
        custom_->setAngle(lc_straighten(a.x(), a.y(), b.x(), b.y(), custom_->angle()));
    };

    auto* side = new QVBoxLayout;
    side->addWidget(ok);
    side->addWidget(cancel);
    side->addSpacing(10);
    side->addWidget(tabs_, 1);
    auto* sideWidget = new QWidget(this);
    sideWidget->setLayout(side);
    sideWidget->setFixedWidth(300);

    auto* outer = new QHBoxLayout(this);
    outer->addLayout(column);
    outer->addLayout(centre, 1);
    outer->addWidget(sideWidget);

    canvas_->zoomChanged = [this] { updateZoomLabel(); };
    selectTool(RemoveDistortion);
    if (const QScreen* screen = parent ? parent->screen() : QGuiApplication::primaryScreen()) {
        resize(screen->availableGeometry().size() * 0.85);
    }
    pushValues();
    canvas_->reload();
    updateZoomLabel();
}

LensCorrectionDialog::~LensCorrectionDialog()
{
    finish(false);
}

QList<double> LensCorrectionDialog::values() const
{
    return auto_->values() + custom_->values() + auto_->profileValues();
}

bool LensCorrectionDialog::load(const QList<double>& values)
{
    if (!ready_ || values.size() != kSlots) {
        return false;
    }
    const std::vector<double> flat(values.begin(), values.end());
    if (!lc_set(::rust::Slice<const double>(flat.data(), flat.size()))) {
        return false;
    }
    constexpr int profileAt = LensAutoPanel::kSlots + LensCustomPanel::kSlots;
    if (!auto_->setValues(values.mid(0, LensAutoPanel::kSlots), values.mid(profileAt))) {
        return false;
    }
    custom_->setValues(values.mid(LensAutoPanel::kSlots, LensCustomPanel::kSlots));
    pushValues();
    return true;
}

int LensCorrectionDialog::tool() const
{
    return tools_->checkedId();
}

void LensCorrectionDialog::selectTool(int tool)
{
    if (QAbstractButton* button = tools_->button(tool)) {
        button->setChecked(true);
    }
}

QString LensCorrectionDialog::helpText() const
{
    return help_->text();
}

void LensCorrectionDialog::showToolHelp()
{
    for (const ToolInfo& info : kTools) {
        if (info.tool == tool()) {
            help_->setText(QString::fromLatin1(info.hint));
        }
    }
}

bool LensCorrectionDialog::eventFilter(QObject* watched, QEvent* event)
{
    if (auto* widget = qobject_cast<QWidget*>(watched)) {
        if (event->type() == QEvent::Enter) {
            help_->setText(widget->toolTip());
        } else if (event->type() == QEvent::Leave) {
            showToolHelp();
        }
    }
    return QDialog::eventFilter(watched, event);
}

void LensCorrectionDialog::pushValues()
{
    if (!ready_) {
        return;
    }
    const QList<double> current = values();
    const std::vector<double> flat(current.begin(), current.end());
    if (!lc_set(::rust::Slice<const double>(flat.data(), flat.size()))) {
        qWarning().noquote() << "Lens Correction: settings refused";
        return;
    }
    canvas_->refresh();
}

void LensCorrectionDialog::pickGridColor()
{
    const QColor picked =
        ColorPickerDialog::getColor(canvas_->gridColor(), this, QStringLiteral("Grid Color"));
    if (picked.isValid()) {
        canvas_->setGridColor(picked);
        gridColor_->setColor(picked);
    }
}

void LensCorrectionDialog::updateZoomLabel()
{
    const double percent = canvas_->zoom() * 100.0;
    const QString text = percent >= 10.0 ? QString::number(std::lround(percent))
                                         : QString::number(percent, 'f', 2);
    zoomLabel_->setText(text + QLatin1Char('%'));
    setWindowTitle(QStringLiteral("Lens Correction (%1%)").arg(text));
}

void LensCorrectionDialog::finish(bool keep)
{
    if (ready_ && !finished_) {
        committed_.clear();
        for (double v : lc_finish(keep)) {
            committed_.append(v);
        }
        finished_ = true;
    }
}

void LensCorrectionDialog::accept()
{
    finish(true);
    QDialog::accept();
}

void LensCorrectionDialog::reject()
{
    finish(false);
    QDialog::reject();
}

bool LensCorrectionDialog::get(PictureView* view, const QList<double>& initial,
                               QList<double>* out, QWidget* parent)
{
    LensCorrectionDialog dialog(view, parent);
    if (!dialog.ready()) {
        return false;
    }
    // A Smart Object carrying a Lens Correction smart filter prefills from the
    // stored record when the caller passes no settings.
    QList<double> start = initial;
    if (start.isEmpty()) {
        for (double v : lc_stored()) {
            start.append(v);
        }
    }
    // Settings that no longer apply (a profile no longer installed, with no
    // stored coefficients) refuse rather than open on defaults OK would then
    // commit.
    if (!start.isEmpty() && !dialog.load(start)) {
        qWarning().noquote() << "Lens Correction: the last settings no longer apply";
        return false;
    }
    if (runDialog(dialog, parent) != QDialog::Accepted || dialog.committed().size() != kSlots) {
        return false;
    }
    *out = dialog.committed();
    return true;
}

} // namespace pictura

#include "moc_lens_correction_dialog.cpp"
