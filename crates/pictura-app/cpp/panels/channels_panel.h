#pragma once

#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QPixmap>
#include <QtWidgets/QWidget>

class QLabel;
class QListWidget;
class QToolButton;

namespace pictura {

class PictureView;

// Window > Channels (PAN-007): the composite and colour channels of the
// document's mode (RGB, Gray, CMYK, Lab, Multichannel, Indexed, Bitmap,
// Duotone) with greyscale thumbnails and CS6's Ctrl+number labels, then the
// alpha channels (saved selections). The eyes of the working RGB channels hide
// a channel on the canvas live (`channelMaskChanged`); the composite eye shows
// them all, and the last visible channel cannot be hidden. The footer loads an
// alpha channel as a selection, saves the selection as a channel, and makes or
// deletes alpha channels. Ported from photorust's ChannelsPanel.
// ponytail: a CMYK / Lab / other source mode lists its channels with
// thumbnails derived from the RGB working data, and their eyes are disabled;
// alpha-channel eyes (the overlay) and channel targeting for edits are not
// modelled; Lab thumbnails show lightness.
class ChannelsPanel : public QWidget {
    Q_OBJECT

public:
    explicit ChannelsPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    int channelCountForTest() const;
    QString channelNameForTest(int index) const;
    QString channelShortcutForTest(int index) const;
    QPixmap channelThumbnailForTest(int index) const;
    bool channelVisibleForTest(int index) const;
    bool channelEyeEnabledForTest(int index) const;
    void toggleChannelForTest(int index);
    void selectChannelForTest(int index);
    // Hover the row at `index` with `mods` held and return the viewport cursor's
    // shape; also drives the Ctrl select-all overlay.
    int hoverChannelForTest(int index, Qt::KeyboardModifiers mods);
    bool ctrlOverlayVisibleForTest() const;
    // The row widget's surface colour, for the Layers-style restyle check.
    QColor rowSurfaceForTest(int index) const;

signals:
    // The visible working colour channels: bit 0 red, 1 green, 2 blue (0x7 all).
    void channelMaskChanged(int mask);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    struct Row {
        QString name;
        QString shortcut;
        int alpha = -1;          // the alpha channel's index, or -1 for a colour row
        QToolButton* eye = nullptr;
        QLabel* thumbnail = nullptr;
        QWidget* widget = nullptr;  // the row's own widget (for its surface)
    };

    void rebuild(const QString& mode, int alphaCount);
    void addRow(const QString& name, const QString& shortcut, int alpha);
    void refreshThumbnails();
    void toggle(int index);
    void updateFooter();
    void styleRows();
    int selectedAlpha() const;
    void updateCtrlOverlay(int index, const QPoint& pos, Qt::KeyboardModifiers mods);

    PictureView* view_ = nullptr;
    QListWidget* list_ = nullptr;
    QLabel* ctrlOverlay_ = nullptr;
    QList<Row> rows_;
    QString mode_;
    int alphaCount_ = -1;
    int mask_ = 0x7;
    // The colour rows the eyes drive (composite first), 0 when they are disabled.
    int liveRows_ = 0;
    QToolButton* load_ = nullptr;
    QToolButton* save_ = nullptr;
    QToolButton* add_ = nullptr;
    QToolButton* remove_ = nullptr;
};

} // namespace pictura
