#include "channels_panel.h"

#include "icons.h"
#include "theme.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/channels.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kThumbnailSize = 24;

struct ChannelDef {
    const char* name;
    const char* shortcut;
};

// CS6's rows per mode: the composite on Ctrl+2, the channels from Ctrl+3
// (Multichannel has no composite and starts at Ctrl+1).
QList<ChannelDef> channelsForMode(const QString& mode)
{
    if (mode == QLatin1String("grayscale")) {
        return {{"Gray", "Ctrl+2"}};
    }
    if (mode == QLatin1String("cmyk")) {
        return {{"CMYK", "Ctrl+2"},    {"Cyan", "Ctrl+3"},  {"Magenta", "Ctrl+4"},
                {"Yellow", "Ctrl+5"},  {"Black", "Ctrl+6"}};
    }
    if (mode == QLatin1String("lab")) {
        return {{"Lab", "Ctrl+2"}, {"Lightness", "Ctrl+3"}, {"a", "Ctrl+4"}, {"b", "Ctrl+5"}};
    }
    if (mode == QLatin1String("multichannel")) {
        return {{"Cyan", "Ctrl+1"}, {"Magenta", "Ctrl+2"}, {"Yellow", "Ctrl+3"}};
    }
    if (mode == QLatin1String("indexed")) {
        return {{"Index", "Ctrl+2"}};
    }
    if (mode == QLatin1String("bitmap")) {
        return {{"Bitmap", "Ctrl+2"}};
    }
    if (mode == QLatin1String("duotone")) {
        return {{"Duotone", "Ctrl+2"}};
    }
    return {{"RGB", "Ctrl+2"}, {"Red", "Ctrl+3"}, {"Green", "Ctrl+4"}, {"Blue", "Ctrl+5"}};
}

// The ink coverage of `px` as CMYK, each 0..1 (the naive conversion; the
// working data is RGB).
void toCmyk(QRgb px, float cmyk[4])
{
    const float r = qRed(px) / 255.0f;
    const float g = qGreen(px) / 255.0f;
    const float b = qBlue(px) / 255.0f;
    const float k = 1.0f - std::max({r, g, b});
    const float inv = k < 1.0f ? 1.0f / (1.0f - k) : 0.0f;
    cmyk[0] = (1.0f - r - k) * inv;
    cmyk[1] = (1.0f - g - k) * inv;
    cmyk[2] = (1.0f - b - k) * inv;
    cmyk[3] = k;
}

// Row `row`'s value of `px` in `mode`, as Photoshop's channel thumbnails show
// it (ink channels white where there is no ink).
int channelValue(const QString& mode, int row, QRgb px)
{
    if (mode == QLatin1String("rgb") && row >= 1 && row <= 3) {
        return row == 1 ? qRed(px) : row == 2 ? qGreen(px) : qBlue(px);
    }
    const bool cmyk = mode == QLatin1String("cmyk");
    const bool multichannel = mode == QLatin1String("multichannel");
    if ((cmyk && row >= 1) || multichannel) {
        float inks[4];
        toCmyk(px, inks);
        return 255 - qRound(inks[cmyk ? row - 1 : row] * 255.0f);
    }
    return qGray(px);
}

QPixmap thumbnail(const QImage& source, const QString& mode, int row)
{
    if (source.isNull()) {
        return QPixmap();
    }
    const QImage scaled =
        source.scaled(kThumbnailSize, kThumbnailSize, Qt::KeepAspectRatio, Qt::FastTransformation)
            .convertToFormat(QImage::Format_ARGB32);
    QImage gray(scaled.size(), QImage::Format_ARGB32);
    for (int y = 0; y < scaled.height(); ++y) {
        for (int x = 0; x < scaled.width(); ++x) {
            const int v = channelValue(mode, row, scaled.pixel(x, y));
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
    for (QToolButton* button : {load_, save_}) {
        footer->addWidget(button);
    }
    footer->addStretch(1);
    for (QToolButton* button : {add_, remove_}) {
        footer->addWidget(button);
    }
    layout->addLayout(footer);

    connect(list_, &QListWidget::currentRowChanged, this, &ChannelsPanel::updateFooter);
    connect(load_, &QToolButton::clicked, this, [this]() {
        const int alpha = selectedAlpha();
        if (view_ && alpha >= 0) {
            view_->load_selection(QStringLiteral("Alpha %1").arg(alpha + 1));
        }
    });
    connect(save_, &QToolButton::clicked, this, [this]() {
        if (view_ && view_->save_selection(QString())) {
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
            refresh();
        }
    });
    rebuild(QStringLiteral("rgb"), 0);
    updateFooter();
}

void ChannelsPanel::addRow(const QString& name, const QString& shortcut, int alpha)
{
    auto* rowWidget = new QWidget(list_);
    rowWidget->setAutoFillBackground(true);
    auto* rowLayout = new QHBoxLayout(rowWidget);
    rowLayout->setContentsMargins(2, 1, 6, 1);
    auto* eye = new QToolButton(rowWidget);
    eye->setCheckable(true);
    eye->setChecked(alpha < 0);
    eye->setAutoRaise(true);
    applyEyeIcon(eye);
    auto* thumb = new QLabel(rowWidget);
    thumb->setFixedSize(kThumbnailSize, kThumbnailSize);
    auto* label = new QLabel(name, rowWidget);
    auto* keys = new QLabel(shortcut, rowWidget);
    keys->setEnabled(false);
    rowLayout->addWidget(eye);
    rowLayout->addWidget(thumb);
    rowLayout->addWidget(label, 1);
    rowLayout->addWidget(keys);
    // Hover is tracked through the row's own widgets: the row covers the item,
    // so the viewport alone would never see the pointer.
    for (QWidget* hovered : {static_cast<QWidget*>(rowWidget), static_cast<QWidget*>(thumb),
                             static_cast<QWidget*>(label), static_cast<QWidget*>(keys)}) {
        hovered->setMouseTracking(true);
        hovered->installEventFilter(this);
    }

    const int index = rows_.size();
    connect(eye, &QToolButton::clicked, this, [this, index]() { toggle(index); });
    auto* item = new QListWidgetItem(list_);
    item->setSizeHint(QSize(0, kThumbnailSize + 8));
    list_->setItemWidget(item, rowWidget);
    rows_.append(Row{name, shortcut, alpha, eye, thumb, rowWidget});
    styleRows();
}

void ChannelsPanel::rebuild(const QString& mode, int alphaCount)
{
    list_->clear();
    rows_.clear();
    const QList<ChannelDef> defs = channelsForMode(mode);
    for (const ChannelDef& def : defs) {
        addRow(QLatin1String(def.name), QLatin1String(def.shortcut), -1);
    }
    // Alpha channels follow on the next Ctrl+number, as far as Ctrl+9 goes.
    int key = QString::fromLatin1(defs.last().shortcut).back().digitValue() + 1;
    for (int i = 0; i < alphaCount; ++i, ++key) {
        addRow(QStringLiteral("Alpha %1").arg(i + 1),
               key <= 9 ? QStringLiteral("Ctrl+%1").arg(key) : QString(), i);
    }
    // Only the working modes' channels can be hidden on the canvas; a lone
    // channel has nothing to hide behind.
    liveRows_ = mode == QLatin1String("rgb") ? 4 : 0;
    for (int i = 0; i < rows_.size(); ++i) {
        const bool live = i < liveRows_;
        rows_[i].eye->setEnabled(live);
        if (!live) {
            rows_[i].eye->setToolTip(rows_[i].alpha >= 0
                                         ? QStringLiteral("Alpha channel overlays are not "
                                                          "implemented yet")
                                         : QStringLiteral("Only the working RGB channels can "
                                                          "be hidden"));
        }
    }
    mode_ = mode;
    alphaCount_ = alphaCount;
    if (mask_ != 0x7) {
        mask_ = 0x7;
        emit channelMaskChanged(mask_);
    }
    list_->setCurrentRow(0);
}

void ChannelsPanel::setView(PictureView* view)
{
    if (view != view_) {
        // Another document: rebuild its rows, which shows all its channels.
        alphaCount_ = -1;
    }
    view_ = view;
    refresh();
}

void ChannelsPanel::refresh()
{
    const bool document = view_ && view_->has_document();
    const QString mode = document ? QString(channels_mode(*view_)) : QStringLiteral("rgb");
    const int alphas = document ? view_->selection_channel_count() : 0;
    if (mode != mode_ || alphas != alphaCount_) {
        rebuild(mode, alphas);
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
        for (int level = view_->display_level_count() - 1; level >= 0 && source.isNull(); --level) {
            const QStringList parts =
                view_->display_level_size(level).split(QLatin1Char(' '), Qt::SkipEmptyParts);
            const int w = parts.value(0).toInt();
            const int h = parts.value(1).toInt();
            if (parts.size() == 2 && qMax(w, h) >= 64) {
                source = view_->display_image(level, 0, 0, w, h);
            }
        }
        if (source.isNull()) {
            source = view_->image();
        }
    }
    for (int i = 0; i < rows_.size(); ++i) {
        const Row& row = rows_.at(i);
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
    if (index < 0 || index >= liveRows_) {
        return;
    }
    if (index == 0) {
        // The composite eye shows every channel.
        mask_ = 0x7;
    } else {
        const int bit = 1 << (index - 1);
        // The last visible channel stays visible.
        if (mask_ != bit) {
            mask_ ^= bit;
        }
    }
    for (int i = 0; i < liveRows_; ++i) {
        const bool on = i == 0 ? mask_ == 0x7 : (mask_ & (1 << (i - 1))) != 0;
        rows_[i].eye->setChecked(on);
        applyEyeIcon(rows_[i].eye);
    }
    emit channelMaskChanged(mask_);
}

int ChannelsPanel::selectedAlpha() const
{
    const int row = list_->currentRow();
    return row >= 0 && row < rows_.size() ? rows_.at(row).alpha : -1;
}

void ChannelsPanel::updateFooter()
{
    const bool document = view_ && view_->has_document();
    const bool alpha = selectedAlpha() >= 0;
    load_->setEnabled(document && alpha);
    remove_->setEnabled(document && alpha);
    add_->setEnabled(document);
    save_->setEnabled(document && view_->has_selection());
    styleRows();
}

// The Channels rows reuse the Layers surface ramp: the selected row steps three
// shades off the window, the rest one, so both panels read alike.
void ChannelsPanel::styleRows()
{
    const QColor base = palette().color(QPalette::Window);
    const int current = list_->currentRow();
    for (int i = 0; i < rows_.size(); ++i) {
        QWidget* widget = rows_.at(i).widget;
        if (!widget) {
            continue;
        }
        QPalette rowPalette = widget->palette();
        rowPalette.setColor(QPalette::Window, Theme::shade(base, i == current ? 3 : 1));
        widget->setPalette(rowPalette);
    }
}

bool ChannelsPanel::eventFilter(QObject* watched, QEvent* event)
{
    auto* widget = qobject_cast<QWidget*>(watched);
    const bool inList = widget
        && (widget == list_->viewport() || list_->viewport()->isAncestorOf(widget));
    if (inList) {
        if (event->type() == QEvent::MouseMove || event->type() == QEvent::HoverMove) {
            auto* mouse = static_cast<QMouseEvent*>(event);
            const QPoint pos = widget->mapTo(list_->viewport(), mouse->position().toPoint());
            QListWidgetItem* item = list_->itemAt(pos);
            const int row = item ? list_->row(item) : -1;
            list_->viewport()->setCursor(row >= 0 ? Qt::PointingHandCursor : Qt::ArrowCursor);
            updateCtrlOverlay(row, pos, mouse->modifiers());
        } else if (event->type() == QEvent::Leave) {
            list_->viewport()->unsetCursor();
            if (ctrlOverlay_) {
                ctrlOverlay_->hide();
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

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

bool ChannelsPanel::ctrlOverlayVisibleForTest() const
{
    return ctrlOverlay_ && !ctrlOverlay_->isHidden();
}

QColor ChannelsPanel::rowSurfaceForTest(int index) const
{
    if (index < 0 || index >= rows_.size() || !rows_.at(index).widget) {
        return {};
    }
    return rows_.at(index).widget->palette().color(QPalette::Window);
}

} // namespace pictura
