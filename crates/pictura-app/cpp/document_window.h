#pragma once

#include <QtCore/QList>
#include <QtCore/QPointer>
#include <QtCore/QString>

class QFontMetrics;
class QWidget;

namespace pictura {

class BrushTipPreview;
class CanvasScrollBars;
class ImageView;
class LoadingCard;
class PictureView;
struct DocWindow;

enum class OpenKind { Psd, Raster, SmartObject, Pdf, PdfPage, Eps };

// One open document. The windows that show it hold the per-viewport state.
struct DocEntry {
    PictureView* view = nullptr;
    QString path;
    // Display name for a path-less import (Open As Smart Object, PDF/EPS
    // import); preferred over the generated Untitled-N name while `path`
    // stays empty.
    QString displayName;
    // The path Open Recent should remember for a path-less import.
    QString recentPath;
    int untitledNumber = 0;
    bool loading = false;
    quint64 token = 0;
    OpenKind kind = OpenKind::Psd;
    // The user selected another tab while this one loaded; do not steal the
    // activation when it finishes.
    bool userMovedAway = false;
    // The document that was active when this open started; a failure or
    // cancel restores it. Guarded because the user may close it mid-load.
    QPointer<PictureView> previousActiveView;
    // In ordinal order: window i is labelled `:i+1` from the second on.
    QList<DocWindow*> windows;
};

// One viewport onto a document: its canvas, scroll bars, and the host it
// lives in.
struct DocWindow {
    enum class Host { Tab, Float, Tile };

    DocEntry* doc = nullptr;
    ImageView* canvas = nullptr;
    CanvasScrollBars* host = nullptr;
    BrushTipPreview* tip = nullptr;
    // The widget the host holds. A ready window's page is its canvas host; a
    // loading window's page is a placeholder until finishLoadingDocument adds
    // the canvas host to it.
    QWidget* page = nullptr;
    // The empty-workspace progress card while loading; null once filled.
    QPointer<LoadingCard> card;
    Host kind = Host::Tab;
};

// The document's name: its display name, file name, or `Untitled-N`.
QString documentDisplayName(const DocEntry& doc);

// The window's unelided label: name, `:N` ordinal, zoom, and mode/bits. A
// loading document's label is its name alone.
QString fullLabel(const DocWindow* window);

// `label` elided to the document tab's text width, with the modified marker.
QString documentTabText(const QString& label, bool dirty, const QFontMetrics& metrics);

} // namespace pictura
