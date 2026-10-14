#include "dialog_chrome.h"

#include "icons.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPointer>
#include <QtGui/QMouseEvent>
#include <QtGui/QWindow>
#include <QtWidgets/QApplication>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLayout>
#include <QtWidgets/QToolButton>

namespace pictura {
namespace {

constexpr int kResizeBorder = 5;
// The layout's own top margin before the title bar was added to it; marks a
// layout already shifted (QMessageBox replaces its layout as its text changes).
constexpr char kBaseTopProperty[] = "picturaChromeBaseTop";
constexpr char kEdgeCursorProperty[] = "picturaChromeEdgeCursor";

class DialogTitleBar : public QWidget {
public:
    explicit DialogTitleBar(QDialog* dialog) : QWidget(dialog)
    {
        setObjectName(QStringLiteral("dialogTitleBar"));
        setAttribute(Qt::WA_StyledBackground, true);
        auto* layout = new QHBoxLayout(this);
        layout->setContentsMargins(8, 2, 2, 2);
        layout->setSpacing(4);
        label_ = new QLabel(dialog->windowTitle(), this);
        label_->setObjectName(QStringLiteral("dialogTitleLabel"));
        layout->addWidget(label_, 1);
        auto* close = new QToolButton(this);
        close->setObjectName(QStringLiteral("dialogCloseButton"));
        close->setIcon(icon(QStringLiteral("panel.close")));
        close->setIconSize(QSize(12, 12));
        close->setAutoRaise(true);
        close->setFocusPolicy(Qt::NoFocus);
        close->setToolTip(QCoreApplication::translate("DialogChrome", "Close"));
        layout->addWidget(close);
        connect(close, &QToolButton::clicked, dialog, &QDialog::reject);
    }

    void setTitle(const QString& title) { label_->setText(title); }

protected:
    void mousePressEvent(QMouseEvent* event) override
    {
        if (event->button() != Qt::LeftButton) {
            QWidget::mousePressEvent(event);
            return;
        }
        // The window system's move works on Wayland too, where a client move()
        // of a top-level is ignored; platforms without it (offscreen) move by hand.
        QWindow* handle = window()->windowHandle();
        if (handle && handle->startSystemMove()) {
            return;
        }
        dragging_ = true;
        dragOffset_ = event->globalPosition().toPoint() - window()->pos();
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (dragging_) {
            window()->move(event->globalPosition().toPoint() - dragOffset_);
        }
    }

    void mouseReleaseEvent(QMouseEvent* event) override
    {
        dragging_ = false;
        QWidget::mouseReleaseEvent(event);
    }

private:
    QLabel* label_ = nullptr;
    bool dragging_ = false;
    QPoint dragOffset_;
};

DialogTitleBar* titleBarOf(const QDialog* dialog)
{
    // Only applyChrome() creates a child with this name.
    return static_cast<DialogTitleBar*>(dialog->findChild<QWidget*>(
        QStringLiteral("dialogTitleBar"), Qt::FindDirectChildrenOnly));
}

QWidget* outlineOf(const QDialog* dialog)
{
    return dialog->findChild<QWidget*>(QStringLiteral("dialogFrame"), Qt::FindDirectChildrenOnly);
}

bool wantsChrome(const QDialog* dialog)
{
    if (!dialog->isWindow() || dialog->windowFlags().testFlag(Qt::FramelessWindowHint)) {
        return false;
    }
    const Qt::WindowType type = dialog->windowType();
    if (type != Qt::Dialog && type != Qt::Window && type != Qt::Sheet && type != Qt::Tool) {
        return false;
    }
    // A platform-drawn picker keeps the platform's frame. Qt builds a picker's
    // widgets (and may polish it) in the constructor when no native helper
    // exists, before any DontUseNativeDialog option is set, so the widget tree
    // itself says whether Qt draws it.
    if (const auto* files = qobject_cast<const QFileDialog*>(dialog)) {
        return files->testOption(QFileDialog::DontUseNativeDialog)
            || files->findChild<QDialogButtonBox*>();
    }
    if (const auto* colors = qobject_cast<const QColorDialog*>(dialog)) {
        return colors->testOption(QColorDialog::DontUseNativeDialog)
            || colors->findChild<QDialogButtonBox*>();
    }
    return true;
}

bool resizable(const QDialog* dialog)
{
    const QLayout* layout = dialog->layout();
    return dialog->minimumSize() != dialog->maximumSize()
        && !(layout && layout->sizeConstraint() == QLayout::SetFixedSize);
}

void syncMargin(QDialog* dialog, const DialogTitleBar* bar)
{
    QLayout* layout = dialog->layout();
    if (!layout || layout->property(kBaseTopProperty).isValid()) {
        return;
    }
    QMargins margins = layout->contentsMargins();
    layout->setProperty(kBaseTopProperty, margins.top());
    margins.setTop(margins.top() + bar->sizeHint().height());
    layout->setContentsMargins(margins);
}

void placeChrome(QDialog* dialog)
{
    DialogTitleBar* bar = titleBarOf(dialog);
    QWidget* outline = outlineOf(dialog);
    if (!bar || !outline) {
        return;
    }
    bar->setGeometry(0, 0, dialog->width(), bar->sizeHint().height());
    bar->raise();
    outline->setGeometry(dialog->rect());
    outline->raise();
}

void applyChrome(QDialog* dialog)
{
    if (titleBarOf(dialog) || !wantsChrome(dialog)) {
        return;
    }
    // Polish precedes the native window's creation, so the flag change does
    // not re-create or hide anything.
    dialog->setWindowFlag(Qt::FramelessWindowHint, true);
    dialog->setMouseTracking(true);
    auto* bar = new DialogTitleBar(dialog);
    auto* outline = new QWidget(dialog);
    outline->setObjectName(QStringLiteral("dialogFrame"));
    outline->setAttribute(Qt::WA_StyledBackground, true);
    outline->setAttribute(Qt::WA_TransparentForMouseEvents, true);
    syncMargin(dialog, bar);
    placeChrome(dialog);
}

Qt::Edges edgesAt(const QDialog* dialog, const QPoint& pos)
{
    Qt::Edges edges;
    if (pos.x() < kResizeBorder) {
        edges |= Qt::LeftEdge;
    } else if (pos.x() >= dialog->width() - kResizeBorder) {
        edges |= Qt::RightEdge;
    }
    if (pos.y() < kResizeBorder) {
        edges |= Qt::TopEdge;
    } else if (pos.y() >= dialog->height() - kResizeBorder) {
        edges |= Qt::BottomEdge;
    }
    return edges;
}

Qt::CursorShape cursorFor(Qt::Edges edges)
{
    const bool horizontal = edges & (Qt::LeftEdge | Qt::RightEdge);
    const bool vertical = edges & (Qt::TopEdge | Qt::BottomEdge);
    if (horizontal && vertical) {
        const bool falling = edges == (Qt::LeftEdge | Qt::TopEdge)
                          || edges == (Qt::RightEdge | Qt::BottomEdge);
        return falling ? Qt::SizeFDiagCursor : Qt::SizeBDiagCursor;
    }
    return horizontal ? Qt::SizeHorCursor : Qt::SizeVerCursor;
}

void clearEdgeCursor(QDialog* dialog)
{
    if (dialog->property(kEdgeCursorProperty).toBool()) {
        dialog->unsetCursor();
        dialog->setProperty(kEdgeCursorProperty, false);
    }
}

// ponytail: only the dialog's own margin band resizes; a child flush with an
// edge (a zero-margin layout) hides that edge from the filter.
bool handleResizeEdges(QDialog* dialog, QEvent* event)
{
    if (event->type() == QEvent::Leave) {
        clearEdgeCursor(dialog);
        return false;
    }
    auto* mouse = static_cast<QMouseEvent*>(event);
    const Qt::Edges edges = resizable(dialog) ? edgesAt(dialog, mouse->position().toPoint())
                                              : Qt::Edges();
    if (event->type() == QEvent::MouseMove && mouse->buttons() == Qt::NoButton) {
        if (edges) {
            dialog->setCursor(cursorFor(edges));
            dialog->setProperty(kEdgeCursorProperty, true);
        } else {
            clearEdgeCursor(dialog);
        }
        return false;
    }
    if (event->type() == QEvent::MouseButtonPress && mouse->button() == Qt::LeftButton && edges) {
        QWindow* handle = dialog->windowHandle();
        return handle && handle->startSystemResize(edges);
    }
    return false;
}

class DialogChromeFilter : public QObject {
public:
    using QObject::QObject;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        switch (event->type()) {
        case QEvent::Polish:
        case QEvent::Show:
        case QEvent::Resize:
        case QEvent::WindowTitleChange:
        case QEvent::MouseMove:
        case QEvent::MouseButtonPress:
        case QEvent::Leave:
            break;
        default:
            return false;
        }
        if (!watched->isWidgetType() || !static_cast<QWidget*>(watched)->isWindow()) {
            return false;
        }
        auto* dialog = qobject_cast<QDialog*>(watched);
        if (!dialog) {
            return false;
        }
        if (event->type() == QEvent::Polish) {
            applyChrome(dialog);
            return false;
        }
        DialogTitleBar* bar = titleBarOf(dialog);
        if (!bar) {
            return false;
        }
        switch (event->type()) {
        case QEvent::Show:
            syncMargin(dialog, bar);
            placeChrome(dialog);
            return false;
        case QEvent::Resize:
            placeChrome(dialog);
            return false;
        case QEvent::WindowTitleChange:
            bar->setTitle(dialog->windowTitle());
            return false;
        default:
            return handleResizeEdges(dialog, event);
        }
    }
};

} // namespace

void installDialogChrome()
{
    static QPointer<DialogChromeFilter> filter;
    if (filter || !qApp) {
        return;
    }
    filter = new DialogChromeFilter(qApp);
    qApp->installEventFilter(filter);
}

QWidget* dialogTitleBar(const QDialog* dialog)
{
    return dialog ? titleBarOf(dialog) : nullptr;
}

} // namespace pictura
