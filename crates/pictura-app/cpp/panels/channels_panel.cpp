#include "channels_panel.h"
#include "channel_drag.h"
#include "channels_rows.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"
#include <QtCore/QScopedValueRollback>

#include "api/document_api.h"
#include "color_models.h"
#include "icons.h"
#include "theme.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QMimeData>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QDrag>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPaintEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cstddef>


namespace pictura {

namespace {

constexpr int kThumbnailSize = 24;

// The `alpha` sentinel for the active layer's mask channel row.
constexpr int kMaskChannel = -2;

// The panel can live in a floated group, where window() is the float rather
// than the frame that owns the status bar.
QMainWindow* mainWindowOf(QWidget* within)
{
    if (auto* main = qobject_cast<QMainWindow*>(within ? within->window() : nullptr)) {
        return main;
    }
    for (QWidget* widget = within; widget; widget = widget->parentWidget()) {
        if (auto* main = qobject_cast<QMainWindow*>(widget)) {
            return main;
        }
    }
    for (QWidget* top : QApplication::topLevelWidgets()) {
        if (auto* main = qobject_cast<QMainWindow*>(top)) {
            return main;
        }
    }
    return nullptr;
}

QPixmap thumbnail(const QImage& source, const QString& mode, int row)
{
    if (source.isNull()) {
        return QPixmap();
    }
    const QImage scaled =
        source.scaled(kThumbnailSize, kThumbnailSize, Qt::KeepAspectRatio, Qt::FastTransformation)
            .convertToFormat(QImage::Format_ARGB32);
    // Photoshop's channel thumbnails: ink rows are white where there is no ink.
    const QVector<uchar> plane = channelPlane(scaled, mode, row);
    QImage gray(scaled.size(), QImage::Format_ARGB32);
    for (int y = 0; y < scaled.height(); ++y) {
        for (int x = 0; x < scaled.width(); ++x) {
            const int v = plane.at(y * scaled.width() + x);
            gray.setPixel(x, y, qRgb(v, v, v));
        }
    }
    return QPixmap::fromImage(gray);
}

void applyEyeIcon(QToolButton* eye)
{
    eye->setIcon(icon(eye->isChecked() ? QStringLiteral("layers.eyeOn")
                                       : QStringLiteral("layers.eyeOff")));
}

// The row surface is painted here rather than through the widget palette: a
// style or palette change re-polishes item widgets and resets an explicit
// palette, but the family-aware fill must survive that. The labels keep the
// inherited palette, so their text follows the family too.
class ChannelRowWidget : public QWidget {
public:
    using QWidget::QWidget;

    void setSurface(const QColor& color)
    {
        surface_ = color;
        update();
    }
    QColor surface() const { return surface_; }
    void setGutter(const QColor& color) { gutter_ = color; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.fillRect(rect(), surface_);
        painter.fillRect(QRect(0, 0, 30, height()), gutter_);
        painter.fillRect(QRect(29, 0, 1, height()), palette().color(QPalette::Base).darker(115));
    }

private:
    QColor surface_;
    QColor gutter_;
};

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

} // namespace

ChannelsPanel::ChannelsPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("channelList"));
    list_->viewport()->setMouseTracking(true);
    list_->viewport()->installEventFilter(this);
    layout->addWidget(list_, 1);

    // The Ctrl-over-channel "select all pixels" symbol: a dashed square pinned
    // at the cursor over the row, transparent to mouse input.
    ctrlOverlay_ = new QLabel(list_->viewport());
    ctrlOverlay_->setObjectName(QStringLiteral("channelsCtrlOverlay"));
    ctrlOverlay_->setAttribute(Qt::WA_TransparentForMouseEvents);
    ctrlOverlay_->setFixedSize(20, 20);
    ctrlOverlay_->setPixmap(icon(QStringLiteral("select.all")).pixmap(20, 20));
    ctrlOverlay_->hide();

    auto* footer = new QHBoxLayout();
    footer->setContentsMargins(4, 2, 4, 2);
    load_ = footerButton(this, "path.loadSelection", QStringLiteral("Load channel as selection"),
                         "channelsLoadSelection");
    save_ = footerButton(this, "layers.mask", QStringLiteral("Save selection as channel"),
                         "channelsSaveSelection");
    add_ = footerButton(this, "layers.newLayer", QStringLiteral("Create new channel"),
                        "channelsNew");
    remove_ = footerButton(this, "layers.delete", QStringLiteral("Delete current channel"),
                           "channelsDelete");
    load_->setAcceptDrops(true);
    add_->setAcceptDrops(true);
    add_->setToolTip(QCoreApplication::translate("PicturaUi", "Create new channel — drop a channel here to duplicate it"));
    for (QToolButton* button : {load_, save_, add_}) {
        button->installEventFilter(this);
    }
    for (QToolButton* button : {load_, save_}) {
        footer->addWidget(button);
    }
    footer->addStretch(1);
    for (QToolButton* button : {add_, remove_}) {
        footer->addWidget(button);
    }
    layout->addLayout(footer);

    connect(list_, &QListWidget::currentRowChanged, this, [this](int row) {
        updateFooter();
        if (!syncing_) {
            applyChannelSolo(row);
            emit channelTargetChanged();
        }
    });
    connect(load_, &QToolButton::clicked, this,
            [this]() { loadRowSelection(list_->currentRow(), Qt::NoModifier); });
    connect(save_, &QToolButton::clicked, this, [this]() {
        if (!view_ || !view_->has_selection()) {
            return;
        }
        if (!channels_can_add(*view_)) {
            if (QMainWindow* main = mainWindowOf(window())) {
                main->statusBar()->showMessage(
                    tr("Cannot save selection: the document already has %1 channels.")
                        .arg(channels_max()),
                    6000);
            }
            return;
        }
        if (view_->save_selection(QString())) {
            refresh();
        }
    });
    connect(add_, &QToolButton::clicked, this, [this]() {
        if (view_ && channels_new_alpha(*view_) >= 0) {
            refresh();
            list_->setCurrentRow(list_->count() - 1);
        }
    });
    connect(remove_, &QToolButton::clicked, this, [this]() {
        const int alpha = selectedAlpha();
        if (view_ && alpha >= 0 && channels_delete_alpha(*view_, alpha)) {
            if (!shownAlphas_.isEmpty()) {
                QSet<int> shifted;
                for (int shown : shownAlphas_) {
                    if (shown != alpha) {
                        shifted.insert(shown > alpha ? shown - 1 : shown);
                    }
                }
                shownAlphas_ = shifted;
                emit alphaVisibilityChanged();
            }
            refresh();
        }
    });
    rebuild(QStringLiteral("rgb"), QStringList(), false);
    updateFooter();
}

void ChannelsPanel::addRow(const QString& name, const QString& shortcut, int alpha, RowKind kind)
{
    auto* rowWidget = new ChannelRowWidget(list_);
    auto* rowLayout = new QHBoxLayout(rowWidget);
    rowLayout->setContentsMargins(0, 2, 6, 2);
    rowLayout->setSpacing(4);
    auto* eye = new QToolButton(rowWidget);
    eye->setFixedWidth(30);
    eye->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Expanding);
    eye->setIconSize(QSize(16, 16));
    eye->setCheckable(true);
    eye->setChecked(alpha < 0 || shownAlphas_.contains(alpha));
    eye->setAutoRaise(true);
    applyEyeIcon(eye);
    auto* thumb = new QLabel(rowWidget);
    thumb->setObjectName(QStringLiteral("channelsRowThumb"));
    thumb->setFixedSize(kThumbnailSize, kThumbnailSize);
    auto* label = new QLabel(name, rowWidget);
    label->setObjectName(QStringLiteral("channelsRowName"));
    auto* keys = new QLabel(shortcut, rowWidget);
    keys->setObjectName(QStringLiteral("channelsRowShortcut"));
    keys->setEnabled(false);
    rowLayout->addWidget(eye);
    rowLayout->addWidget(thumb);
    rowLayout->addWidget(label, 1);
    rowLayout->addWidget(keys);
    // The row covers the item, so let the row container own hover, selection,
    // and drag hit-testing; the text and thumbnail pass clicks through to it.
    // The eye button stays interactive.
    for (QWidget* passive : {static_cast<QWidget*>(thumb), static_cast<QWidget*>(label),
                             static_cast<QWidget*>(keys)}) {
        passive->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    rowWidget->setMouseTracking(true);
    rowWidget->installEventFilter(this);

    const int index = rows_.size();
    connect(eye, &QToolButton::clicked, this, [this, index]() {
        toggle(index);
    });
    auto* item = new QListWidgetItem(list_);
    item->setSizeHint(QSize(0, kThumbnailSize + 4));
    list_->setItemWidget(item, rowWidget);
    rows_.append(Row{name, shortcut, alpha>=0?RowKind::Alpha:(alpha==kMaskChannel?RowKind::LayerMask:kind), alpha, eye, thumb, rowWidget});
    styleRows();
}

void ChannelsPanel::rebuild(const QString& mode, const QStringList& alphaNames, bool storedPlates)
{
    QScopedValueRollback<bool> syncing(syncing_,true);
    quickMask_=view_ && quick_mask_active(*view_);
    quickMaskRowIndex_=-1;
    list_->clear();
    rows_.clear();
    maskRowIndex_ = -1;
    maskName_.clear();
    const QList<ChannelRowDef> defs = channelsForMode(mode, storedPlates);
    for (const ChannelRowDef& def : defs) {
        addRow(QLatin1String(def.name), QLatin1String(def.shortcut), -1);
    }
    // The active layer's raster mask, when it has one, is the next channel row
    // (CS6 lists it after the colour channels, before the alpha channels).
    if (view_ && view_->has_document()) {
        const QString name = QString(channels_active_mask_name(*view_));
        if (!name.isEmpty()) {
            maskName_ = name;
            maskRowIndex_ = rows_.size();
            addRow(name, QStringLiteral("Ctrl+\\"), kMaskChannel);
        }
    }
    // Alpha channels follow on the next Ctrl+number, as far as Ctrl+9 goes.
    int key = defs.isEmpty() ? 1 : QString::fromLatin1(defs.last().shortcut).back().digitValue() + 1;
    for (int i = 0; i < alphaNames.size(); ++i, ++key) {
        addRow(alphaNames.value(i, QStringLiteral("Alpha %1").arg(i + 1)),
               key <= 9 ? QStringLiteral("Ctrl+%1").arg(key) : QString(), i);
    }
    if(quickMask_) {
        quickMaskRowIndex_=rows_.size();
        addRow(QStringLiteral("Quick Mask"),QString(),-1,RowKind::QuickMask);
    }
    // Only the working modes' channels can be hidden on the canvas; a lone
    // channel has nothing to hide behind.
    liveRows_ = mode == QLatin1String("rgb") ? 4 : 0;
    for (int i = 0; i < rows_.size(); ++i) {
        const bool live = i < liveRows_;
        const bool alpha = rows_[i].kind == RowKind::Alpha;
        const bool enabled = live || alpha;
        rows_[i].eye->setEnabled(enabled);
        // A disabled eye is only a status icon; let the click select the row
        // instead of being swallowed by the disabled button.
        rows_[i].eye->setAttribute(Qt::WA_TransparentForMouseEvents, !enabled);
        if (alpha) {
            rows_[i].eye->setToolTip(QCoreApplication::translate("PicturaUi", "Show this alpha channel on the canvas"));
        } else if (!live) {
            rows_[i].eye->setToolTip(
                QCoreApplication::translate("PicturaUi", "Only the working RGB channels can be hidden"));
        }
    }
    mode_ = mode;
    storedPlates_ = storedPlates;
    alphaCount_ = alphaNames.size();
    alphaNames_ = alphaNames;
    if (mask_ != 0x7) {
        mask_ = 0x7;
        emit channelMaskChanged(mask_);
    }
    // Keep the mask channel selected while it is soloed; otherwise show the
    // composite row. `syncing_` stops this from ending the solo.
    const bool soloed = maskRowIndex_ >= 0 && view_ && view_->has_document()
                        && !mask_solo_get(*view_).isEmpty();
    syncing_ = true;
    list_->setCurrentRow(soloed ? maskRowIndex_ : 0);
    syncing_ = false;
}

void ChannelsPanel::setView(PictureView* view)
{
    if (view != view_) {
        // Another document: rebuild its rows, which shows all its channels.
        alphaCount_ = -1;
        shownAlphas_.clear();
    }
    view_ = view;
    refresh();
}

void ChannelsPanel::refresh()
{
    const bool document = view_ && view_->has_document();
    const QString mode = document ? QString(channels_mode(*view_)) : QStringLiteral("rgb");
    const QStringList alphaNames = document ? channels_alpha_names(*view_) : QStringList();
    const bool storedPlates = document && channels_multichannel_plates(*view_);
    for (auto it = shownAlphas_.begin(); it != shownAlphas_.end();) {
        if (*it >= alphaNames.size()) {
            it = shownAlphas_.erase(it);
        } else {
            ++it;
        }
    }
    const QString maskName = document ? QString(channels_active_mask_name(*view_)) : QString();
    if (mode != mode_ || alphaNames.size() != alphaCount_ || alphaNames != alphaNames_
        || storedPlates != storedPlates_ || maskName != maskName_
        || quickMask_ != (document && quick_mask_active(*view_))) {
        rebuild(mode, alphaNames, storedPlates);
    }
    refreshThumbnails();
    updateFooter();
}

void ChannelsPanel::refreshThumbnails()
{
    QImage source;
    if (view_ && view_->has_document()) {
        // Thumbnails need ~24 px: take the coarsest pyramid level of at least
        // 64 px rather than the full composite.
        const QVector<document_api::CanvasSize> levels = document_api::pyramidSizes(*view_);
        for (int level = levels.size() - 1; level >= 0 && source.isNull(); --level) {
            const document_api::CanvasSize& size = levels.at(level);
            if (qMax(size.width, size.height) >= 64) {
                source = view_->display_image(level, 0, 0, size.width, size.height);
            }
        }
        if (source.isNull()) {
            source = view_->image();
        }
    }
    for (int i = 0; i < rows_.size(); ++i) {
        const Row& row = rows_.at(i);
        if(row.kind==RowKind::QuickMask) {
            row.thumbnail->setPixmap(QPixmap::fromImage(quick_mask_thumbnail(*view_,kThumbnailSize)));
            continue;
        }
        if (row.alpha == kMaskChannel) {
            int w = 0;
            int h = 0;
            const ::rust::Vec<std::uint8_t> plane =
                view_ ? channels_active_mask_thumbnail(*view_, kThumbnailSize, w, h)
                      : ::rust::Vec<std::uint8_t>();
            QImage gray(w, h, QImage::Format_Grayscale8);
            for (int y = 0; y < h && int(plane.size()) == w * h; ++y) {
                std::copy_n(plane.data() + size_t(y) * size_t(w), w, gray.scanLine(y));
            }
            row.thumbnail->setPixmap(QPixmap::fromImage(gray));
            continue;
        }
        if (row.alpha < 0) {
            row.thumbnail->setPixmap(thumbnail(source, mode_, i));
            continue;
        }
        int w = 0;
        int h = 0;
        const ::rust::Vec<std::uint8_t> plane =
            view_ ? channels_alpha_thumbnail(*view_, row.alpha, kThumbnailSize, w, h)
                  : ::rust::Vec<std::uint8_t>();
        QImage gray(w, h, QImage::Format_Grayscale8);
        for (int y = 0; y < h && int(plane.size()) == w * h; ++y) {
            std::copy_n(plane.data() + size_t(y) * size_t(w), w, gray.scanLine(y));
        }
        row.thumbnail->setPixmap(QPixmap::fromImage(gray));
    }
}

void ChannelsPanel::toggle(int index)
{
    if (index < 0) {
        return;
    }
    if (index >= liveRows_) {
        if (index < rows_.size() && rows_.at(index).kind == RowKind::Alpha) {
            toggleAlpha(rows_.at(index).alpha);
        }
        return;
    }
    if (!shownAlphas_.isEmpty()) {
        shownAlphas_.clear();
        updateAlphaEyes();
        emit alphaVisibilityChanged();
    }
    int nextMask = mask_;
    if (index == 0) {
        // The composite eye shows every channel.
        nextMask = 0x7;
    } else {
        const int bit = 1 << (index - 1);
        // The last visible channel stays visible.
        if (mask_ != bit) {
            nextMask ^= bit;
        }
    }
    setRgbVisibility(nextMask);
}

void ChannelsPanel::setRgbVisibility(int mask)
{
    mask_ = mask;
    for (int i = 0; i < liveRows_; ++i) {
        const bool on = i == 0 ? mask_ == 0x7 : (mask_ & (1 << (i - 1))) != 0;
        rows_[i].eye->setChecked(on);
        applyEyeIcon(rows_[i].eye);
    }
    emit channelMaskChanged(mask_);
}

void ChannelsPanel::selectRow(int index)
{
    list_->setCurrentRow(index);
    if (index < 0 || index >= liveRows_) {
        return;
    }
    if (index == 0) {
        setRgbVisibility(0x7);
    } else {
        const int bit = 1 << (index - 1);
        if (mask_ == 0x7 || !(mask_ & bit)) {
            setRgbVisibility(bit);
        }
    }
    styleRows();
}

void ChannelsPanel::toggleAlpha(int alpha)
{
    if (alpha < 0) {
        return;
    }
    if (shownAlphas_.contains(alpha)) {
        shownAlphas_.remove(alpha);
    } else {
        shownAlphas_.insert(alpha);
    }
    updateAlphaEyes();
    emit alphaVisibilityChanged();
}

void ChannelsPanel::updateAlphaEyes()
{
    for (Row& row : rows_) {
        if (row.kind == RowKind::Alpha) {
            row.eye->setChecked(shownAlphas_.contains(row.alpha));
            applyEyeIcon(row.eye);
        }
    }
}

QList<int> ChannelsPanel::shownAlphas() const { return shownAlphas_.values(); }

int ChannelsPanel::selectedAlpha() const
{
    const int row = list_->currentRow();
    return row >= 0 && row < rows_.size() ? rows_.at(row).alpha : -1;
}

int ChannelsPanel::maskRowIndexForTest() const { return maskRowIndex_; }

void ChannelsPanel::applyChannelSolo(int row)
{
    // Selecting the mask channel shows that layer's mask on the canvas (the
    // view-only solo); selecting any other row restores the composite.
    if (!view_ || !view_->has_document()) {
        return;
    }
    if(row>=0 && row==quickMaskRowIndex_) {quick_mask_set_grayscale(*view_,true);return;}
    if(quick_mask_active(*view_)) quick_mask_set(*view_,false);
    if (row >= 0 && row == maskRowIndex_) {
        mask_solo_set(*view_, layer_selection_anchor(*view_));
    } else {
        mask_solo_set(*view_, QString());
    }
}

void ChannelsPanel::updateFooter()
{
    const bool document = view_ && view_->has_document();
    const bool alpha = selectedAlpha() >= 0;
    const int row = list_->currentRow();
    const bool loadable = document && row >= 0 && row < rows_.size()
        && (rows_.at(row).kind == RowKind::Color || rows_.at(row).kind == RowKind::Alpha);
    load_->setEnabled(loadable);
    remove_->setEnabled(document && alpha);
    const bool temporary=list_->currentRow()>=0 && list_->currentRow()==quickMaskRowIndex_;
    const bool room = document && channels_can_add(*view_);
    add_->setEnabled(document && !temporary && room);
    add_->setToolTip(room || !document || temporary
                         ? tr("Create new channel")
                         : tr("Cannot create a channel: the document already has %1 channels.")
                               .arg(channels_max()));
    save_->setEnabled(document && view_->has_selection() && !temporary);
    styleRows();
}

// The Channels rows reuse the Layers surface ramp: the selected row steps three
// shades off the window, the rest one, so both panels read alike.
void ChannelsPanel::styleRows()
{
    const QColor base = palette().color(QPalette::Window);
    const int current = list_->currentRow();
    for (int i = 0; i < rows_.size(); ++i) {
        auto* widget = static_cast<ChannelRowWidget*>(rows_.at(i).widget);
        if (!widget) {
            continue;
        }
        const bool selected = i == current
            || (mode_ == QLatin1String("rgb") && current == 0 && i > 0 && i < liveRows_);
        widget->setSurface(Theme::contrastShade(base, selected ? 3 : 1));
        widget->setGutter(Theme::contrastShade(base, selected ? 3 : 2));
    }
}

void ChannelsPanel::changeEvent(QEvent* event)
{
    QWidget::changeEvent(event);
    if (event->type() == QEvent::PaletteChange || event->type() == QEvent::StyleChange) {
        styleRows();
    }
}

bool ChannelsPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == load_) {
        if (event->type() == QEvent::DragEnter || event->type() == QEvent::DragMove) {
            auto* drag = static_cast<QDropEvent*>(event);
            if (drag->mimeData()->hasFormat(QString::fromLatin1(kChannelRowMime))) {
                drag->acceptProposedAction();
                return true;
            }
        } else if (event->type() == QEvent::Drop) {
            auto* drop = static_cast<QDropEvent*>(event);
            const ChannelDrag drag = channelDrag(drop->mimeData());
            auto* source = qobject_cast<PictureView*>(drag.sourceGuard.data());
            if (!source || source != view_) {
                drop->ignore();
                return true;
            }
            loadRowSelection(drag.row, Qt::NoModifier);
            drop->acceptProposedAction();
            return true;
        }
        return QWidget::eventFilter(watched, event);
    }
    if (watched == add_) {
        if (event->type() == QEvent::DragEnter || event->type() == QEvent::DragMove) {
            auto* drag = static_cast<QDropEvent*>(event);
            if (drag->mimeData()->hasFormat(QString::fromLatin1(kChannelRowMime))) {
                drag->acceptProposedAction();
                return true;
            }
        } else if (event->type() == QEvent::Drop) {
            auto* drop = static_cast<QDropEvent*>(event);
            if (duplicateChannelFromMime(drop->mimeData())) {
                drop->acceptProposedAction();
            } else {
                drop->ignore();
            }
            return true;
        }
        return QWidget::eventFilter(watched, event);
    }
    if (watched == save_) {
        if (event->type() == QEvent::MouseButtonPress) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            if (mouse->button() == Qt::LeftButton && (mouse->modifiers() & Qt::AltModifier)) {
                emit saveOptionsRequested();
                return true;
            }
        }
        return QWidget::eventFilter(watched, event);
    }
    auto* widget = qobject_cast<QWidget*>(watched);
    const bool inList = widget
        && (widget == list_->viewport() || list_->viewport()->isAncestorOf(widget));
    if (inList) {
        if (event->type() == QEvent::MouseMove || event->type() == QEvent::HoverMove) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            const QPoint pos = widget->mapTo(list_->viewport(), mouse->position().toPoint());
            const int row = rowAt(pos);
            list_->viewport()->setCursor(row >= 0 ? Qt::PointingHandCursor : Qt::ArrowCursor);
            updateCtrlOverlay(row, pos, mouse->modifiers());
            if ((mouse->buttons() & Qt::LeftButton) && dragRow_ >= 0
                && (pos - dragStart_).manhattanLength()
                    >= QApplication::startDragDistance()) {
                const int dragged = dragRow_;
                dragRow_ = -1;
                startChannelDrag(dragged);
                return true;
            }
        } else if (event->type() == QEvent::MouseButtonDblClick) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            const QPoint pos = widget->mapTo(list_->viewport(), mouse->position().toPoint());
            const int row = rowAt(pos);
            const int alpha = alphaIndexAt(row);
            if (mouse->button() == Qt::LeftButton && alpha >= 0 && view_
                && !channels_alpha_is_spot(*view_, alpha)) {
                list_->setCurrentRow(row);
                emit channelOptionsRequested();
                return true;
            }
        } else if (event->type() == QEvent::MouseButtonPress) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            const QPoint pos = widget->mapTo(list_->viewport(), mouse->position().toPoint());
            dragRow_ = rowAt(pos);
            dragStart_ = pos;
            if (mouse->button() == Qt::LeftButton && dragRow_ >= 0) {
                if (mouse->modifiers() & Qt::ControlModifier) {
                    list_->setCurrentRow(dragRow_);
                } else {
                    selectRow(dragRow_);
                }
            }
            if (mouse->button() == Qt::LeftButton && (mouse->modifiers() & Qt::ControlModifier)) {
                loadRowSelection(dragRow_, mouse->modifiers());
            }
        } else if (event->type() == QEvent::MouseButtonRelease) {
            dragRow_ = -1;
        } else if (event->type() == QEvent::Leave) {
            list_->viewport()->unsetCursor();
            if (ctrlOverlay_) {
                ctrlOverlay_->hide();
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

int ChannelsPanel::rowAt(const QPoint& viewportPos) const
{
    QListWidgetItem* item = list_->itemAt(viewportPos);
    return item ? list_->row(item) : -1;
}

void ChannelsPanel::loadRowSelection(int row, Qt::KeyboardModifiers mods)
{
    if(row>=0 && row==quickMaskRowIndex_) return;
    if (!view_ || row < 0 || row >= rows_.size()) {
        return;
    }
    QString mode = QStringLiteral("new");
    if (mods & Qt::ControlModifier) {
        const bool shift = mods.testFlag(Qt::ShiftModifier);
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (shift && alt) {
            mode = QStringLiteral("intersect");
        } else if (shift) {
            mode = QStringLiteral("add");
        } else if (alt) {
            mode = QStringLiteral("subtract");
        }
    }
    const int alpha = rows_.at(row).alpha;
    if (alpha >= 0) {
        load_selection_target(*view_, QStringLiteral("alpha:%1").arg(alpha), false, mode);
    } else if (alpha == -1) {
        // The mask row (kMaskChannel) keeps its view-only solo, not a load.
        channels_load_color_selection(*view_, row, mask_, false, mode);
    }
}

void ChannelsPanel::startChannelDrag(int row)
{
    QMimeData* mime = channelDragMime(row);
    if (!mime) {
        return;
    }
    auto* drag = new QDrag(list_->viewport());
    drag->setMimeData(mime);
    drag->exec(Qt::CopyAction);
}

QToolButton* ChannelsPanel::eyeForTest(int row) const
{
    return row >= 0 && row < rows_.size() ? rows_.at(row).eye : nullptr;
}

QWidget* ChannelsPanel::rowWidgetForTest(int row) const
{
    return row >= 0 && row < rows_.size() ? rows_.at(row).widget : nullptr;
}

QMimeData* ChannelsPanel::channelDragMime(int row) const
{
    if (!view_ || !canDuplicateRow(row)) {
        return nullptr;
    }
    const Row& candidate = rows_.at(row);
    return makeChannelDragMime(view_, row, candidate.alpha, mask_, candidate.name);
}

bool ChannelsPanel::duplicateChannelFromMime(const QMimeData* mime)
{
    const ChannelDrag drag = channelDrag(mime);
    auto* source = qobject_cast<PictureView*>(drag.sourceGuard.data());
    if (!source || !source->has_document() || !view_ || !view_->has_document()
        || drag.row < 0) {
        return false;
    }
    const QString sourceTarget = drag.alpha >= 0
        ? QStringLiteral("alpha:%1").arg(drag.alpha)
        : QStringLiteral("color:%1").arg(drag.row);
    const QString name = drag.name.isEmpty() ? tr("Alpha 1 copy") : tr("%1 copy").arg(drag.name);
    const bool duplicated = source == view_
        ? duplicate_channel_in_view(*view_, sourceTarget, drag.mask, name, false)
        : transfer_duplicate_channel(*view_, *source, sourceTarget, drag.mask, name, false);
    if (!duplicated) {
        return false;
    }
    refresh();
    return true;
}

int ChannelsPanel::currentRow() const { return list_->currentRow(); }

int ChannelsPanel::alphaIndexAt(int row) const
{
    return row >= 0 && row < rows_.size() ? rows_.at(row).alpha : -1;
}

QString ChannelsPanel::nameAt(int row) const
{
    return row >= 0 && row < rows_.size() ? rows_.at(row).name : QString();
}

bool ChannelsPanel::canDuplicateRow(int row) const
{
    if (row < 0 || row >= rows_.size()) {
        return false;
    }
    const RowKind kind = rows_.at(row).kind;
    return kind == RowKind::Color || kind == RowKind::Alpha;
}

bool ChannelsPanel::canDuplicateCurrent() const
{
    return view_ && view_->has_document() && canDuplicateRow(currentRow());
}

int ChannelsPanel::channelMask() const { return mask_; }

void ChannelsPanel::updateCtrlOverlay(int index, const QPoint& pos, Qt::KeyboardModifiers mods)
{
    if (!ctrlOverlay_) {
        return;
    }
    if (index < 0 || !(mods & Qt::ControlModifier)) {
        ctrlOverlay_->hide();
        return;
    }
    ctrlOverlay_->move(pos + QPoint(2, 2));
    ctrlOverlay_->show();
    ctrlOverlay_->raise();
}

int ChannelsPanel::channelCountForTest() const { return rows_.size(); }

QString ChannelsPanel::channelNameForTest(int index) const
{
    return index >= 0 && index < rows_.size() ? rows_.at(index).name : QString();
}

QString ChannelsPanel::channelShortcutForTest(int index) const
{
    return index >= 0 && index < rows_.size() ? rows_.at(index).shortcut : QString();
}

QPixmap ChannelsPanel::channelThumbnailForTest(int index) const
{
    if (index < 0 || index >= rows_.size()) {
        return QPixmap();
    }
    const QPixmap pixmap = rows_.at(index).thumbnail->pixmap();
    return pixmap.isNull() || pixmap.width() == 0 ? QPixmap() : pixmap;
}

bool ChannelsPanel::channelVisibleForTest(int index) const
{
    return index >= 0 && index < rows_.size() && rows_.at(index).eye->isChecked();
}

bool ChannelsPanel::channelEyeEnabledForTest(int index) const
{
    return index >= 0 && index < rows_.size() && rows_.at(index).eye->isEnabled();
}

void ChannelsPanel::toggleChannelForTest(int index) { toggle(index); }

bool ChannelsPanel::toggleChannelForShortcut(int digit)
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    for (int i = 0; i < rows_.size(); ++i) {
        const QString& shortcut = rows_.at(i).shortcut;
        if (!shortcut.isEmpty() && shortcut.back().digitValue() == digit) {
            toggle(i);
            return true;
        }
    }
    return false;
}

void ChannelsPanel::selectChannelForTest(int index) { list_->setCurrentRow(index); }

int ChannelsPanel::hoverChannelForTest(int index, Qt::KeyboardModifiers mods)
{
    if (index < 0 || index >= rows_.size()) {
        return -1;
    }
    const QPoint pos = list_->visualItemRect(list_->item(index)).center();
    QMouseEvent move(QEvent::MouseMove, pos, list_->viewport()->mapToGlobal(pos), Qt::NoButton,
                     Qt::NoButton, mods);
    QCoreApplication::sendEvent(list_->viewport(), &move);
    return static_cast<int>(list_->viewport()->cursor().shape());
}

void ChannelsPanel::clickChannelForTest(int index, Qt::KeyboardModifiers mods)
{
    if (index < 0 || index >= rows_.size()) {
        return;
    }
    const QPoint pos = list_->visualItemRect(list_->item(index)).center();
    QMouseEvent press(QEvent::MouseButtonPress, pos, list_->viewport()->mapToGlobal(pos),
                      Qt::LeftButton, Qt::LeftButton, mods);
    QCoreApplication::sendEvent(list_->viewport(), &press);
}

int ChannelsPanel::currentChannelForTest() const { return list_->currentRow(); }

bool ChannelsPanel::ctrlOverlayVisibleForTest() const
{
    return ctrlOverlay_ && !ctrlOverlay_->isHidden();
}

QColor ChannelsPanel::rowSurfaceForTest(int index) const
{
    if (index < 0 || index >= rows_.size() || !rows_.at(index).widget) {
        return {};
    }
    return static_cast<const ChannelRowWidget*>(rows_.at(index).widget)->surface();
}

QColor ChannelsPanel::rowTextColorForTest(int index) const
{
    if (index < 0 || index >= rows_.size() || !rows_.at(index).widget) {
        return {};
    }
    return rows_.at(index).widget->palette().color(QPalette::WindowText);
}

} // namespace pictura

#include "moc_channels_panel.cpp"
