#pragma once

#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QStringList>

class QMimeData;
class QWidget;

namespace pictura {

class PicturaMainWindow;
class PictureView;

// Routes an OS file drag over the frame's drop targets: a document ImageView
// places each file into the active document, every other watched target opens
// each file as a new tab. A Layers-panel drag from another document copies the
// layer into the active one (hovering a tab activates it). Any other drag is
// never consumed, so the in-panel drag and tab reordering keep their handlers.
class FileDropRouter : public QObject {
    Q_OBJECT

public:
    explicit FileDropRouter(PicturaMainWindow* frame);

    bool eventFilter(QObject* watched, QEvent* event) override;

    // The local regular files carried by `mime`; directories and non-file URLs
    // are discarded and a URL-less drag yields an empty list.
    static QStringList localPaths(const QMimeData* mime);
    // The document tab under the global point `globalPos` when it is not the
    // current document's, else -1 (also off the document tab bar).
    static int otherDocumentTabAt(const QPoint& globalPos);
    // Whether `globalPos` lies over another document's tab or canvas, for a
    // selection drag that starts on `source`. `within` is the source canvas.
    static bool otherDocumentAt(const QPoint& globalPos, const PictureView* source,
                                QWidget* within);

private:
    // The Layers-panel drag branch; false when `event` carries no layer drag.
    bool routeLayerDrag(QObject* watched, QEvent* event);
    // The Channels-panel and selection-outline drag branches.
    bool routeChannelDrag(QObject* watched, QEvent* event);
    bool routeSelectionDrag(QObject* watched, QEvent* event);
    // The document under a drag event: the hovered tab activates its document;
    // a canvas resolves to its own document. Null when the target is unknown.
    PictureView* dragTarget(QObject* watched, QEvent* event);

    PicturaMainWindow* frame_ = nullptr;
};

} // namespace pictura
