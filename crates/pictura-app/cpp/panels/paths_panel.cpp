#include "paths_panel.h"

#include "icons.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/path_list.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"
#include "tools.h"

#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPixmap>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <string>

namespace pictura {

namespace {

constexpr int kThumbHeight = 32;

// The targeted path as one QPainterPath in document pixels.
QPainterPath targetPath(const PictureView& v)
{
    QPainterPath out;
    const int subpaths = path_subpath_count(v);
    for (int sp = 0; sp < subpaths; ++sp) {
        const int count = path_point_count(v, sp);
        const int segments = path_subpath_closed(v, sp) ? count : count - 1;
        ::rust::Vec<double> a = path_point(v, sp, 0);
        if (a.size() != 9) {
            continue;
        }
        out.moveTo(a[0], a[1]);
        for (int i = 0; i < segments; ++i) {
            const ::rust::Vec<double> b = path_point(v, sp, (i + 1) % count);
            if (b.size() != 9) {
                break;
            }
            // A missing handle is reported at its anchor, so this is a plain
            // cubic either way.
            out.cubicTo(a[6], a[7], b[3], b[4], b[0], b[1]);
            a = b;
        }
    }
    return out;
}

// Panel row `row`'s outline, from its flattened polyline.
QPainterPath outlinePath(const PictureView& v, int row)
{
    QPainterPath out;
    const ::rust::Vec<double> flat = paths_outline(v, row);
    std::size_t at = 0;
    while (at < flat.size()) {
        const auto count = static_cast<std::size_t>(flat[at++]);
        for (std::size_t i = 0; i < count && at + 1 < flat.size(); ++i, at += 2) {
            if (i == 0) {
                out.moveTo(flat[at], flat[at + 1]);
            } else {
                out.lineTo(flat[at], flat[at + 1]);
            }
        }
    }
    return out;
}

// CS6's path thumbnail: the canvas as a white card with the path drawn on it.
QPixmap thumbnail(const PictureView& v, const QPainterPath& path)
{
    const double w = qMax(1, v.document_width());
    const double h = qMax(1, v.document_height());
    const double scale = kThumbHeight / h;
    QPixmap pixmap(qMax(1, qRound(w * scale)), kThumbHeight);
    pixmap.fill(Qt::white);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.scale(scale, scale);
    painter.setPen(QPen(Qt::black, 1.0 / scale));
    painter.drawPath(path);
    return pixmap;
}

QToolButton* footerButton(QWidget* parent, const char* iconId, const QString& tip,
                          const char* name)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(QLatin1String(name));
    button->setIcon(icon(QLatin1String(iconId)));
    button->setToolTip(tip);
    button->setAutoRaise(true);
    return button;
}

std::uint32_t argb(const QColor& color) { return color.rgba(); }

} // namespace

PathsPanel::PathsPanel(QWidget* parent)
    : QWidget(parent)
{
    setWindowTitle(tr("Paths"));
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("pathsList"));
    list_->setIconSize(QSize(kThumbHeight * 2, kThumbHeight));
    list_->setSelectionMode(QAbstractItemView::SingleSelection);
    list_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    list_->setContextMenuPolicy(Qt::CustomContextMenu);
    list_->viewport()->installEventFilter(this);
    layout->addWidget(list_, 1);
    empty_ = new QLabel(tr("No Paths"), this);
    empty_->setAlignment(Qt::AlignCenter);
    empty_->setEnabled(false);
    layout->addWidget(empty_, 1, Qt::AlignCenter);

    // CS6's footer order, left to right.
    auto* footer = new QHBoxLayout();
    footer->setContentsMargins(4, 2, 4, 2);
    fill_ = footerButton(this, "path.fill", tr("Fill path with foreground color"), "pathsFill");
    stroke_ = footerButton(this, "path.stroke", tr("Stroke path with brush"), "pathsStroke");
    load_ = footerButton(this, "path.loadSelection", tr("Load path as a selection"),
                         "pathsLoadSelection");
    // Tracing a selection into anchors is its own piece of work; the button
    // keeps the panel's shape, disabled, like other unimplemented controls.
    makeWork_ = footerButton(this, "path.makeWorkPath", tr("Not implemented yet"),
                             "pathsMakeWorkPath");
    makeWork_->setEnabled(false);
    add_ = footerButton(this, "path.newPath", tr("Create new path"), "pathsNew");
    remove_ = footerButton(this, "path.delete", tr("Delete current path"), "pathsDelete");
    for (QToolButton* button : {fill_, stroke_, load_, makeWork_}) {
        footer->addWidget(button);
    }
    footer->addStretch(1);
    for (QToolButton* button : {add_, remove_}) {
        footer->addWidget(button);
    }
    layout->addLayout(footer);

    connect(list_, &QListWidget::itemSelectionChanged, this, &PathsPanel::onSelectionChanged);
    connect(list_, &QListWidget::itemChanged, this, &PathsPanel::onItemChanged);
    connect(list_, &QListWidget::itemDoubleClicked, this, &PathsPanel::onDoubleClicked);
    connect(list_, &QListWidget::customContextMenuRequested, this, &PathsPanel::onContextMenu);
    connect(fill_, &QToolButton::clicked, this, [this]() { fillPath(); });
    connect(stroke_, &QToolButton::clicked, this, [this]() { strokePath(); });
    connect(load_, &QToolButton::clicked, this, [this]() { loadSelection(); });
    connect(add_, &QToolButton::clicked, this, [this]() { newPath(); });
    connect(remove_, &QToolButton::clicked, this, [this]() { deletePath(); });
    refresh();
}

void PathsPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void PathsPanel::refresh()
{
    updating_ = true;
    list_->clear();
    if (view_ && view_->has_document()) {
        PictureView& v = *view_;
        const bool layerTarget = path_layer_target(v);
        path_set_layer_target(v, true);
        if (path_target_is_layer(v)) {
            const QString name = QString::fromStdString(std::string(path_target_name(v)));
            auto* item = new QListWidgetItem(QIcon(thumbnail(v, targetPath(v))),
                                             tr("%1 Shape Path").arg(name), list_);
            item->setData(Qt::UserRole, kShapeRow);
        }
        path_set_layer_target(v, layerTarget);
        auto addRow = [&](int row, const QString& name) {
            auto* item = new QListWidgetItem(QIcon(thumbnail(v, outlinePath(v, row))), name,
                                             list_);
            item->setData(Qt::UserRole, row);
            return item;
        };
        const int saved = paths_saved_count(v);
        for (int i = 0; i < saved; ++i) {
            QListWidgetItem* item = addRow(i, paths_saved_name(v, i));
            item->setFlags(item->flags() | Qt::ItemIsEditable);
        }
        if (paths_has_work_path(v)) {
            // CS6 sets the temporary Work Path's name in italics.
            QListWidgetItem* item = addRow(kWorkRow, tr("Work Path"));
            QFont font = item->font();
            font.setItalic(true);
            item->setFont(font);
        }
        // With no panel path active, an active shape layer's row is selected.
        const int active = paths_active(v);
        const int current = active == -2 ? kShapeRow : active;
        for (int i = 0; i < list_->count(); ++i) {
            if (list_->item(i)->data(Qt::UserRole).toInt() == current) {
                list_->setCurrentRow(i);
            }
        }
    }
    const bool has = list_->count() > 0;
    list_->setVisible(has);
    empty_->setVisible(!has);
    updating_ = false;
    updateFooter();
}

int PathsPanel::selectedRow() const
{
    QListWidgetItem* item = list_->currentItem();
    return item && item->isSelected() ? item->data(Qt::UserRole).toInt() : -2;
}

template <typename Command>
bool PathsPanel::onSelected(Command command)
{
    const int row = selectedRow();
    if (!view_ || !view_->has_document() || row == -2) {
        return false;
    }
    PictureView& v = *view_;
    const bool layerTarget = path_layer_target(v);
    path_set_layer_target(v, row == kShapeRow);
    const bool done = command(v);
    path_set_layer_target(v, layerTarget);
    return done;
}

void PathsPanel::onSelectionChanged()
{
    if (updating_ || !view_ || !view_->has_document()) {
        return;
    }
    const int row = selectedRow();
    // The shape row's outline is the shape layer's own; no panel path is shown.
    paths_set_active(*view_, row == kShapeRow ? -2 : row);
    updateFooter();
}

void PathsPanel::onItemChanged(QListWidgetItem* item)
{
    if (updating_ || !view_ || !item) {
        return;
    }
    const int row = item->data(Qt::UserRole).toInt();
    if (row >= 0 && !paths_rename(*view_, row, item->text())) {
        refresh();
    }
}

void PathsPanel::onDoubleClicked(QListWidgetItem* item)
{
    if (!view_ || !item) {
        return;
    }
    const int row = item->data(Qt::UserRole).toInt();
    if (row >= 0) {
        list_->editItem(item);
    } else if (row == kWorkRow) {
        bool ok = false;
        const QString name = QInputDialog::getText(this, tr("Save Path"), tr("Name:"),
                                                   QLineEdit::Normal,
                                                   paths_next_name(*view_), &ok);
        if (ok) {
            saveWorkPath(name);
        }
    }
}

void PathsPanel::onContextMenu(const QPoint& pos)
{
    QListWidgetItem* item = list_->itemAt(pos);
    if (!view_ || !item) {
        return;
    }
    list_->setCurrentItem(item);
    const int row = item->data(Qt::UserRole).toInt();
    QMenu menu(this);
    QAction* save = row == kWorkRow ? menu.addAction(tr("Save Path...")) : nullptr;
    QAction* duplicate = row >= 0 ? menu.addAction(tr("Duplicate Path")) : nullptr;
    QAction* remove = row != kShapeRow ? menu.addAction(tr("Delete Path")) : nullptr;
    menu.addSeparator();
    QAction* select = menu.addAction(tr("Make Selection..."));
    QAction* fill = menu.addAction(tr("Fill Path"));
    QAction* stroke = menu.addAction(tr("Stroke Path"));
    QAction* chosen = menu.exec(list_->viewport()->mapToGlobal(pos));
    if (!chosen) {
        return;
    }
    if (chosen == save) {
        onDoubleClicked(item);
    } else if (chosen == duplicate) {
        duplicatePath();
    } else if (chosen == remove) {
        deletePath();
    } else if (chosen == select) {
        bool ok = false;
        const double feather = QInputDialog::getDouble(
            this, tr("Make Selection"), tr("Feather Radius (pixels):"), 0.0, 0.0, 250.0, 1, &ok);
        if (ok) {
            loadSelection(feather);
        }
    } else if (chosen == fill) {
        fillPath();
    } else if (chosen == stroke) {
        strokePath();
    }
}

bool PathsPanel::newPath()
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const bool made = paths_new(*view_, paths_next_name(*view_)) >= 0;
    refresh();
    return made;
}

bool PathsPanel::deletePath()
{
    const int row = selectedRow();
    if (!view_ || (row != kWorkRow && row < 0)) {
        return false;
    }
    const bool deleted = paths_delete(*view_, row);
    refresh();
    return deleted;
}

bool PathsPanel::duplicatePath()
{
    const int row = selectedRow();
    if (!view_ || row < 0) {
        return false;
    }
    const bool copied = paths_duplicate(*view_, row) >= 0;
    refresh();
    return copied;
}

bool PathsPanel::saveWorkPath(const QString& name)
{
    if (!view_) {
        return false;
    }
    const bool saved = paths_save_work(*view_, name) >= 0;
    refresh();
    return saved;
}

bool PathsPanel::fillPath()
{
    const QColor foreground = tools_ ? tools_->foreground() : QColor(Qt::black);
    return onSelected([&](PictureView& v) { return paths_fill(v, argb(foreground)); });
}

bool PathsPanel::strokePath()
{
    const QColor foreground = tools_ ? tools_->foreground() : QColor(Qt::black);
    const auto* controller = dynamic_cast<const ToolController*>(tools_);
    PaintOptions* paint = controller ? controller->optionsFor<PaintOptions>(ToolId::Brush)
                                     : nullptr;
    const int size = paint ? paint->size() : 5;
    const int hardness = paint ? paint->hardness() : 100;
    const int opacity = paint ? paint->opacity() : 100;
    const int flow = paint ? paint->flow() : 100;
    const int spacing = paint ? paint->spacing() : 25;
    return onSelected([&](PictureView& v) {
        return paths_stroke(v, argb(foreground), size, hardness, opacity, flow, spacing);
    });
}

bool PathsPanel::loadSelection(double feather)
{
    return onSelected([&](PictureView& v) {
        return paths_make_selection(v, feather, QStringLiteral("new"));
    });
}

void PathsPanel::updateFooter()
{
    const int row = selectedRow();
    const bool has = view_ && view_->has_document();
    for (QToolButton* button : {fill_, stroke_, load_}) {
        button->setEnabled(has && row != -2);
    }
    add_->setEnabled(has);
    remove_->setEnabled(has && (row >= 0 || row == kWorkRow));
}

// CS6 hides every path when the empty part of the panel is clicked.
bool PathsPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == list_->viewport() && event->type() == QEvent::MouseButtonPress) {
        const auto* press = static_cast<QMouseEvent*>(event);
        if (!list_->itemAt(press->position().toPoint())) {
            list_->clearSelection();
            list_->setCurrentRow(-1);
            if (view_ && view_->has_document()) {
                paths_set_active(*view_, -2);
            }
            updateFooter();
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura

#include "moc_paths_panel.cpp"
