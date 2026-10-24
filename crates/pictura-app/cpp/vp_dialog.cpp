#include "vp_dialog.h"
#include "dialogs.h"
#include "frame.h"
#include "icons.h"
#include "vp_canvas.h"
#include "vp_options.h"

#include <QtCore/QDebug>
#include <QtCore/QSignalBlocker>
#include <QtGui/QAction>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeySequence>
#include <QtGui/QScreen>
#include <QtGui/QShortcut>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <functional>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point_retouch.cxxqt.h"

namespace pictura {

namespace {

struct VpToolInfo {
    VanishingPointDialog::Tool tool;
    const char* icon;
    const char* name;
    const char* help;
    Qt::Key key;
};

const VpToolInfo kTools[] = {
    {VanishingPointDialog::EditPlane, "tool.pathselection", "Edit Plane Tool",
     "Select a plane, drag its corners to reshape it, its edge nodes to resize it, or its inside "
     "to move it. Ctrl-drag an edge node for a related plane.",
     Qt::Key_V},
    {VanishingPointDialog::CreatePlane, "tool.createplane", "Create Plane Tool",
     "Click four corners to define a plane. Ctrl-drag an edge node to tear off a related plane.",
     Qt::Key_C},
    {VanishingPointDialog::Marquee, "tool.marquee", "Marquee Tool",
     "Drag a selection in perspective; double-click to select a plane. Alt-drag to move a copy, "
     "Ctrl-drag to fill it from under the pointer.",
     Qt::Key_M},
    {VanishingPointDialog::Stamp, "tool.clonestamp", "Stamp Tool",
     "Alt-click to set the source, then paint to clone it in perspective.", Qt::Key_S},
    {VanishingPointDialog::Brush, "tool.brush", "Brush Tool",
     "Paint with Brush Color in perspective.", Qt::Key_B},
    {VanishingPointDialog::Transform, "tool.objectscale", "Transform Tool",
     "Drag a corner to scale (Shift keeps the aspect), just past one to rotate, inside to move.",
     Qt::Key_T},
    {VanishingPointDialog::Eyedropper, "tool.eyedropper", "Eyedropper Tool",
     "Click to pick Brush Color from the preview.", Qt::Key_I},
    {VanishingPointDialog::Hand, "tool.hand", "Hand Tool", "Drag to move the view.", Qt::Key_H},
    {VanishingPointDialog::Zoom, "tool.zoom", "Zoom Tool",
     "Click to zoom in; Alt-click to zoom out. Hold X to zoom temporarily.", Qt::Key_Z},
};

} // namespace

VanishingPointDialog::VanishingPointDialog(PictureView* view, const QString& documentName,
                                           QWidget* parent)
    : QDialog(parent), view_(view), documentName_(documentName)
{
    setObjectName(QStringLiteral("vpDialog"));
    ready_ = view_ && vp_begin(*view_);
    options_ = new VpOptionsBar(this);
    if (auto* frame = qobject_cast<PicturaMainWindow*>(parent)) {
        options_->setColour(frame->foregroundColor());
    }

    // The menu button and the tool options.
    auto* menuButton = new QToolButton(this);
    menuButton->setObjectName(QStringLiteral("vpMenuButton"));
    menuButton->setText(QStringLiteral("≡"));
    menuButton->setToolTip(QStringLiteral("Vanishing Point settings and commands"));
    menuButton->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(menuButton);
    menu->setObjectName(QStringLiteral("vpMenu"));
    showEdges_ = menu->addAction(QStringLiteral("Show Edges"));
    showEdges_->setObjectName(QStringLiteral("vpShowEdges"));
    showEdges_->setCheckable(true);
    showEdges_->setChecked(true);
    showEdges_->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_H));
    showEdges_->setShortcutContext(Qt::WindowShortcut);
    addAction(showEdges_);
    // CS6 asks for the grid render anew each session.
    renderGrids_ = menu->addAction(QStringLiteral("Render Grids To Photoshop"));
    renderGrids_->setObjectName(QStringLiteral("vpRenderGrids"));
    renderGrids_->setCheckable(true);
    menuButton->setMenu(menu);

    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);

    auto* options = new QHBoxLayout;
    options->addWidget(menuButton);
    options->addSpacing(8);
    options->addWidget(options_, 1);
    options->addWidget(ok);
    options->addWidget(cancel);

    help_ = new QLabel(this);
    help_->setObjectName(QStringLiteral("vpHelp"));

    // The tool column.
    tools_ = new QButtonGroup(this);
    tools_->setExclusive(true);
    auto* column = new QVBoxLayout;
    column->setSpacing(2);
    for (const VpToolInfo& info : kTools) {
        if (info.tool == Hand) {
            column->addSpacing(6); // CS6 rules the column before the view tools
        }
        auto* button = new QToolButton(this);
        button->setObjectName(QStringLiteral("vpTool%1").arg(int(info.tool)));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setIcon(icon(QString::fromLatin1(info.icon)));
        button->setIconSize(QSize(20, 20));
        button->setToolTip(QStringLiteral("%1 (%2)").arg(QString::fromLatin1(info.name),
                                                         QKeySequence(info.key).toString()));
        tools_->addButton(button, info.tool);
        column->addWidget(button);
        auto* shortcut = new QShortcut(QKeySequence(info.key), this);
        connect(shortcut, &QShortcut::activated, this, [this, tool = info.tool] { selectTool(tool); });
    }
    column->addStretch(1);

    canvas_ = new VpCanvas(this);
    connect(tools_, &QButtonGroup::idToggled, this, [this](int id, bool on) {
        if (on) {
            toolChanged(id);
        }
    });
    connect(showEdges_, &QAction::toggled, canvas_, &VpCanvas::setShowEdges);

    auto* zoomOut = new QToolButton(this);
    zoomOut->setText(QStringLiteral("−"));
    auto* zoomIn = new QToolButton(this);
    zoomIn->setText(QStringLiteral("+"));
    zoomLabel_ = new QLabel(this);
    zoomLabel_->setObjectName(QStringLiteral("vpZoom"));
    zoomLabel_->setMinimumWidth(56);
    zoomLabel_->setAlignment(Qt::AlignCenter);
    connect(zoomOut, &QToolButton::clicked, this, [this] { canvas_->zoomBy(0.5); });
    connect(zoomIn, &QToolButton::clicked, this, [this] { canvas_->zoomBy(2.0); });
    addZoomShortcuts(this, zoomIn, zoomOut);
    auto* zoomRow = new QHBoxLayout;
    zoomRow->addWidget(zoomOut);
    zoomRow->addWidget(zoomLabel_);
    zoomRow->addWidget(zoomIn);
    zoomRow->addStretch(1);

    auto* centre = new QVBoxLayout;
    centre->addWidget(canvas_, 1);
    centre->addLayout(zoomRow);
    auto* body = new QHBoxLayout;
    body->addLayout(column);
    body->addLayout(centre, 1);
    auto* outer = new QVBoxLayout(this);
    outer->addLayout(options);
    outer->addWidget(help_);
    outer->addLayout(body, 1);

    // The dialog's own undo and redo, pixels included.
    auto shortcut = [this](const QKeySequence& keys, std::function<void()> run) {
        auto* s = new QShortcut(keys, this);
        connect(s, &QShortcut::activated, this, std::move(run));
    };
    shortcut(QKeySequence::Undo, [this] {
        if (vp_undo()) {
            canvas_->refreshAll();
            syncOptions();
        }
    });
    shortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_Z), [this] {
        if (vp_redo()) {
            canvas_->refreshAll();
            syncOptions();
        }
    });
    shortcut(QKeySequence(Qt::CTRL | Qt::Key_V), [this] { paste(); });
    shortcut(QKeySequence(Qt::CTRL | Qt::Key_D), [this] {
        vp_deselect();
        canvas_->refreshAll();
    });
    shortcut(QKeySequence(Qt::CTRL | Qt::Key_T), [this] {
        if (vp_has_float()) {
            selectTool(Transform);
        }
    });
    shortcut(QKeySequence(Qt::Key_BracketLeft), [this] { options_->stepDiameter(tool(), -1); });
    shortcut(QKeySequence(Qt::Key_BracketRight), [this] { options_->stepDiameter(tool(), 1); });

    connect(options_->gridSize(), &QSpinBox::valueChanged, this, [this](int value) {
        vp_set_grid_size(value);
        canvas_->refreshAll();
    });
    connect(options_->angle(), &QSpinBox::valueChanged, this, [this](int value) {
        if (vp_set_angle(value)) {
            canvas_->refreshAll();
        }
    });
    options_->mirrored = [this](bool horizontal) {
        vp_mirror(horizontal);
        canvas_->refreshAll();
    };
    canvas_->planesChanged = [this] { syncOptions(); };
    canvas_->planeCreated = [this] { selectTool(EditPlane); };
    canvas_->zoomChanged = [this] { updateZoomLabel(); };
    canvas_->colourPicked = [this](const QColor& c) { options_->setColour(c); };
    canvas_->notice = [this](const QString& text) { help_->setText(text); };

    selectTool(ready_ && vp_plane_count() > 0 ? EditPlane : CreatePlane);
    if (const QScreen* screen = parent ? parent->screen() : QGuiApplication::primaryScreen()) {
        resize(screen->availableGeometry().size() * 0.85);
    }
    if (ready_) {
        canvas_->reload();
    }
    syncOptions();
    updateZoomLabel();
}

VanishingPointDialog::~VanishingPointDialog()
{
    finish(false);
}

bool VanishingPointDialog::load(const QList<double>& values)
{
    if (!ready_ || values.size() != 1 || !vp_load(static_cast<int>(std::lround(values.at(0))))) {
        return false;
    }
    renderGrids_->setChecked(true);
    canvas_->refreshAll();
    syncOptions();
    selectTool(EditPlane);
    return true;
}

int VanishingPointDialog::tool() const
{
    return tools_->checkedId();
}

void VanishingPointDialog::selectTool(int tool)
{
    if (QAbstractButton* button = tools_->button(tool)) {
        button->setChecked(true);
    }
}

void VanishingPointDialog::toolChanged(int tool)
{
    // A floating selection drops when a tool other than Marquee or
    // Transform takes over.
    if (tool != Marquee && tool != Transform && vp_drop_float()) {
        canvas_->refreshAll();
    }
    canvas_->setMode(static_cast<VpCanvas::Mode>(tool));
    options_->showFor(tool);
    for (const VpToolInfo& info : kTools) {
        if (info.tool == tool) {
            help_->setText(QString::fromLatin1(info.help));
        }
    }
}

bool VanishingPointDialog::paste()
{
    if (!ready_) {
        return false;
    }
    if (auto* frame = qobject_cast<PicturaMainWindow*>(parentWidget())) {
        frame->importSystemClipboard();
    }
    switch (vp_paste_clipboard()) {
    case 0:
        selectTool(Marquee);
        canvas_->refreshAll();
        return true;
    case 1:
        help_->setText(QStringLiteral("Create a plane before pasting."));
        return false;
    default:
        help_->setText(QStringLiteral("There is nothing on the clipboard to paste."));
        return false;
    }
}

void VanishingPointDialog::syncOptions()
{
    const bool active = ready_ && vp_active() >= 0;
    QSpinBox* gridSize = options_->gridSize();
    QSpinBox* angle = options_->angle();
    {
        const QSignalBlocker block(gridSize);
        gridSize->setValue(static_cast<int>(std::lround(vp_grid_size())));
    }
    gridSize->setEnabled(active);
    {
        const QSignalBlocker block(angle);
        angle->setValue(static_cast<int>(std::lround(vp_angle())));
    }
    // A root plane has no angle, and a plane with related planes keeps its own.
    angle->setEnabled(active && vp_angle_editable());
}

void VanishingPointDialog::updateZoomLabel()
{
    const double percent = canvas_->zoom() * 100.0;
    const QString text = percent >= 10.0 ? QString::number(std::lround(percent))
                                         : QString::number(percent, 'f', 2);
    zoomLabel_->setText(text + QLatin1Char('%'));
    setWindowTitle(QStringLiteral("Vanishing Point (%1 @ %2%)").arg(documentName_, text));
}

void VanishingPointDialog::finish(bool keep)
{
    if (ready_ && !finished_) {
        committed_.clear();
        const int id = vp_finish(*view_, keep, renderGrids_->isChecked());
        if (id > 0) {
            committed_.append(id);
        }
        finished_ = true;
    }
}

void VanishingPointDialog::accept()
{
    finish(true);
    QDialog::accept();
}

void VanishingPointDialog::reject()
{
    finish(false);
    QDialog::reject();
}

bool VanishingPointDialog::get(PictureView* view, const QString& documentName,
                               const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    VanishingPointDialog dialog(view, documentName, parent);
    if (!dialog.ready()) {
        return false;
    }
    if (!initial.isEmpty() && !dialog.load(initial)) {
        qWarning().noquote() << "Vanishing Point: the last planes are no longer kept";
        return false;
    }
    if (runDialog(dialog, parent) != QDialog::Accepted || dialog.committed().size() != 1) {
        return false;
    }
    *out = dialog.committed();
    return true;
}

} // namespace pictura

#include "moc_vp_dialog.cpp"
