#include "dialogs.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QEvent>
#include <QtCore/QEventLoop>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QSettings>
#include <QtCore/QStandardPaths>
#include <QtCore/QStorageInfo>
#include <QtCore/QXmlStreamReader>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QShortcut>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {

namespace {

bool g_interactive = true;
UnsavedChoice g_nonInteractiveChoice = UnsavedChoice::Discard;

// A single pixel/background layer is the only shape a flat raster format can
// hold without losing the document's structure.
bool documentIsFlat(PictureView* view)
{
    if (!view || view->layer_count() != 1) {
        return false;
    }
    const QString kind = view->layer_kind(0);
    return kind == QLatin1String("pixel") || kind == QLatin1String("background");
}

// Rolling prefix type-ahead for a non-editable combo: a letter jumps to the
// next item starting with it, and repeating the same letter within Qt's
// keyboard-input interval keeps advancing (wrapping).
class RollingTypeAhead : public QObject {
public:
    explicit RollingTypeAhead(QComboBox* combo)
        : QObject(combo)
        , combo_(combo)
    {
    }

protected:
    bool eventFilter(QObject* obj, QEvent* event) override
    {
        if (event->type() != QEvent::KeyPress) {
            return QObject::eventFilter(obj, event);
        }
        auto* key = static_cast<QKeyEvent*>(event);
        const QString text = key->text();
        if (text.isEmpty() || !text.at(0).isPrint()) {
            return QObject::eventFilter(obj, event);
        }
        const QChar ch = text.at(0);
        const int count = combo_->count();
        const bool rolling = pendingKey_ == ch && rollTimer_.isValid()
            && rollTimer_.elapsed() <= QApplication::keyboardInputInterval();
        const int start = rolling ? combo_->currentIndex() + 1 : 0;
        for (int step = 0; step < count; ++step) {
            const int index = (start + step) % count;
            if (combo_->itemText(index).startsWith(ch, Qt::CaseInsensitive)) {
                combo_->setCurrentIndex(index);
                pendingKey_ = ch;
                rollTimer_.restart();
                return true;
            }
        }
        // No match: let the key through (Space, etc.) instead of swallowing it.
        return QObject::eventFilter(obj, event);
    }

private:
    QComboBox* combo_;
    QChar pendingKey_;
    QElapsedTimer rollTimer_;
};

bool isKnownImageSuffix(const QString& suffix)
{
    static const QStringList known = {QStringLiteral("psd"), QStringLiteral("psb"),
                                      QStringLiteral("png"), QStringLiteral("jpg"),
                                      QStringLiteral("jpeg"), QStringLiteral("jpe"),
                                      QStringLiteral("tif"), QStringLiteral("tiff"),
                                      QStringLiteral("webp"), QStringLiteral("bmp")};
    return known.contains(suffix);
}

} // namespace

void setUnsavedPromptInteractive(bool interactive)
{
    g_interactive = interactive;
}

void setNonInteractiveUnsavedChoice(UnsavedChoice choice)
{
    g_nonInteractiveChoice = choice;
}

QImage documentPreviewImage(PictureView* view, int minEdge)
{
    if (!view || !view->has_document()) {
        return QImage();
    }
    int best = -1;
    QSize size;
    for (int level = 0; level < view->display_level_count(); ++level) {
        const QStringList parts =
            view->display_level_size(level).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (parts.size() != 2) {
            break;
        }
        const QSize levelSize(parts[0].toInt(), parts[1].toInt());
        if (level > 0 && qMax(levelSize.width(), levelSize.height()) < minEdge) {
            break;
        }
        best = level;
        size = levelSize;
    }
    return best < 0 ? QImage() : view->display_image(best, 0, 0, size.width(), size.height());
}

UnsavedChoice askUnsaved(QWidget* parent, const QString& documentName)
{
    if (!g_interactive) {
        return g_nonInteractiveChoice;
    }

    QMessageBox box(QMessageBox::Warning, QStringLiteral("Kooka Pictura"),
                    QStringLiteral("Save changes to \"%1\"?").arg(documentName),
                    QMessageBox::NoButton, parent);
    QPushButton* saveButton = box.addButton(QStringLiteral("Save"), QMessageBox::AcceptRole);
    box.addButton(QStringLiteral("Discard"), QMessageBox::DestructiveRole);
    QPushButton* cancelButton = box.addButton(QStringLiteral("Cancel"), QMessageBox::RejectRole);
    box.setDefaultButton(saveButton);
    box.exec();

    const QAbstractButton* clicked = box.clickedButton();
    if (clicked == saveButton) {
        return UnsavedChoice::Save;
    }
    if (clicked == cancelButton || !clicked) {
        return UnsavedChoice::Cancel;
    }
    return UnsavedChoice::Discard;
}

const QStringList& saveFileFilters()
{
    static const QStringList filters = {
        QStringLiteral("Photoshop (*.PSD)"),
        QStringLiteral("Photoshop Large Format (*.PSB)"),
        QStringLiteral("PNG (*.PNG)"),
        QStringLiteral("JPEG (*.JPG *.JPEG *.JPE)"),
        QStringLiteral("TIFF (*.TIF *.TIFF)"),
        QStringLiteral("WebP (*.WEBP)"),
        QStringLiteral("BMP (*.BMP)"),
        QStringLiteral("All Formats (*.PSD *.PSB *.PNG *.JPG *.JPEG *.JPE *.TIF *.TIFF "
                       "*.WEBP *.BMP)"),
    };
    return filters;
}

const QStringList& openFileFilters()
{
    static const QStringList filters = {
        QStringLiteral("All Formats (*.PSD *.PSB *.PNG *.JPG *.JPEG *.JPE *.GIF *.BMP "
                       "*.TIF *.TIFF *.WEBP)"),
        QStringLiteral("Photoshop (*.PSD)"),
        QStringLiteral("Photoshop Large Format (*.PSB)"),
        QStringLiteral("PNG (*.PNG)"),
        QStringLiteral("JPEG (*.JPG *.JPEG *.JPE)"),
        QStringLiteral("TIFF (*.TIF *.TIFF)"),
        QStringLiteral("WebP (*.WEBP)"),
        QStringLiteral("GIF (*.GIF)"),
        QStringLiteral("BMP (*.BMP)"),
        QStringLiteral("All files (*)"),
    };
    return filters;
}

bool documentRequiresLayers(PictureView* view)
{
    return !documentIsFlat(view);
}

bool savePathNeedsFormatDialog(const QString& path, PictureView* view, bool pathIsNative)
{
    if (path.isEmpty()) {
        return true;
    }
    return view && !pathIsNative && documentRequiresLayers(view);
}

int defaultSaveFilterIndex(PictureView* view)
{
    if (!view || documentRequiresLayers(view)) {
        return 0; // Photoshop (*.PSD)
    }
    const QString format = output_format(*view).toLower();
    if (format == QLatin1String("psb")) {
        return 1;
    }
    if (format == QLatin1String("png")) {
        return 2;
    }
    if (format == QLatin1String("jpg") || format == QLatin1String("jpeg")
        || format == QLatin1String("jpe")) {
        return 3;
    }
    if (format == QLatin1String("tif") || format == QLatin1String("tiff")) {
        return 4;
    }
    if (format == QLatin1String("webp")) {
        return 5;
    }
    if (format == QLatin1String("bmp")) {
        return 6;
    }
    return 0;
}

bool isRasterImageSuffix(const QString& suffix)
{
    static const QStringList raster = {QStringLiteral("png"), QStringLiteral("jpg"),
                                       QStringLiteral("jpeg"), QStringLiteral("jpe"),
                                       QStringLiteral("tif"), QStringLiteral("tiff"),
                                       QStringLiteral("webp"), QStringLiteral("bmp")};
    return raster.contains(suffix.toLower());
}

bool saveAsNeedsFeatureWarning(const QString& path, PictureView* view)
{
    return isRasterImageSuffix(QFileInfo(path).suffix()) && documentRequiresLayers(view);
}

QString defaultSuffixForFilter(int filterIndex)
{
    switch (filterIndex) {
    case 1:
        return QStringLiteral(".psb");
    case 2:
        return QStringLiteral(".png");
    case 3:
        return QStringLiteral(".jpg");
    case 4:
        return QStringLiteral(".tif");
    case 5:
        return QStringLiteral(".webp");
    case 6:
        return QStringLiteral(".bmp");
    default:
        return QStringLiteral(".psd");
    }
}

QString lowercasedImageSuffix(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix();
    if (suffix.isEmpty() || !isKnownImageSuffix(suffix.toLower())) {
        return path;
    }
    QString base = path;
    base.chop(suffix.size());
    return base + suffix.toLower();
}

namespace {

QStringList g_dialogRecentPaths;

// Qt's sidebar accepts only browsable directories; everything else is dropped.
// Dedup is by canonical path so a symlink or bind-mount alias collapses.
void addPlace(QList<QUrl>& urls, QSet<QString>& seen, const QString& path)
{
    if (path.isEmpty()) {
        return;
    }
    const QFileInfo info(QDir::cleanPath(path));
    if (!info.exists() || !info.isDir()) {
        return;
    }
    const QString canonical = info.canonicalFilePath();
    const QString key = canonical.isEmpty() ? info.absoluteFilePath() : canonical;
    if (key.isEmpty() || seen.contains(key)) {
        return;
    }
    seen.insert(key);
    urls.append(QUrl::fromLocalFile(key));
}

void addMountedVolumes(QList<QUrl>& urls, QSet<QString>& seen)
{
    static const QSet<QString> pseudo = {
        QStringLiteral("proc"),       QStringLiteral("sysfs"),
        QStringLiteral("tmpfs"),      QStringLiteral("devtmpfs"),
        QStringLiteral("devpts"),     QStringLiteral("cgroup"),
        QStringLiteral("cgroup2"),    QStringLiteral("overlay"),
        QStringLiteral("squashfs"),   QStringLiteral("ramfs"),
        QStringLiteral("securityfs"), QStringLiteral("debugfs"),
        QStringLiteral("tracefs"),    QStringLiteral("configfs"),
        QStringLiteral("fusectl"),    QStringLiteral("mqueue"),
        QStringLiteral("hugetlbfs"),  QStringLiteral("binfmt_misc"),
        QStringLiteral("pstore"),     QStringLiteral("autofs"),
        QStringLiteral("efivarfs"),   QStringLiteral("nsfs"),
    };
    for (const QStorageInfo& volume : QStorageInfo::mountedVolumes()) {
        if (!volume.isValid() || !volume.isReady()
            || pseudo.contains(QString::fromLatin1(volume.fileSystemType()))) {
            continue;
        }
        addPlace(urls, seen, volume.rootPath());
    }
}

void addStandardLocations(QList<QUrl>& urls, QSet<QString>& seen)
{
    static const QStandardPaths::StandardLocation locations[] = {
        QStandardPaths::HomeLocation,      QStandardPaths::DesktopLocation,
        QStandardPaths::DocumentsLocation, QStandardPaths::DownloadLocation,
        QStandardPaths::PicturesLocation,  QStandardPaths::MusicLocation,
        QStandardPaths::MoviesLocation,
    };
    for (QStandardPaths::StandardLocation location : locations) {
        for (const QString& path : QStandardPaths::standardLocations(location)) {
            addPlace(urls, seen, path);
        }
    }
}

// KDE Places: <bookmark href="file:///...">; the rest (trash:/, remote:/,
// recentlyused:/) are KIO URLs Qt's dialog cannot browse.
void addXbelBookmarks(QList<QUrl>& urls, QSet<QString>& seen, const QString& file)
{
    QFile xbel(file);
    if (!xbel.open(QIODevice::ReadOnly)) {
        return;
    }
    QXmlStreamReader xml(&xbel);
    while (!xml.atEnd()) {
        if (xml.readNext() != QXmlStreamReader::StartElement
            || xml.name() != QLatin1String("bookmark")) {
            continue;
        }
        const QString href = xml.attributes().value(QLatin1String("href")).toString();
        if (href.startsWith(QLatin1String("file://"))) {
            addPlace(urls, seen, QUrl(href).toLocalFile());
        }
    }
}

// GTK places: one "file:///path Label" per line.
void addGtkBookmarks(QList<QUrl>& urls, QSet<QString>& seen, const QString& file)
{
    QFile bookmarks(file);
    if (!bookmarks.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return;
    }
    while (!bookmarks.atEnd()) {
        const QString line = QString::fromUtf8(bookmarks.readLine()).trimmed();
        if (line.isEmpty() || line.startsWith(QLatin1Char('#'))) {
            continue;
        }
        const int space = line.indexOf(QLatin1Char(' '));
        const QString uri = space < 0 ? line : line.left(space);
        if (uri.startsWith(QLatin1String("file://"))) {
            addPlace(urls, seen, QUrl(uri).toLocalFile());
        }
    }
}

} // namespace

QList<QUrl> fileDialogPlaces()
{
    QList<QUrl> urls;
    QSet<QString> seen;
    addPlace(urls, seen, QStringLiteral("/"));
    addMountedVolumes(urls, seen);
    addStandardLocations(urls, seen);
    // XDG-correct lookup so a non-default XDG_DATA_HOME/XDG_CONFIG_HOME works;
    // the system data dirs are included for distribution-wide places.
    for (const QString& dir :
         QStandardPaths::standardLocations(QStandardPaths::GenericDataLocation)) {
        addXbelBookmarks(urls, seen, dir + QStringLiteral("/user-places.xbel"));
    }
    for (const QString& dir :
         QStandardPaths::standardLocations(QStandardPaths::GenericConfigLocation)) {
        addGtkBookmarks(urls, seen, dir + QStringLiteral("/gtk-3.0/bookmarks"));
    }
    for (const QString& path : g_dialogRecentPaths) {
        addPlace(urls, seen, QFileInfo(path).absolutePath());
    }
    return urls;
}

void setFileDialogRecentPaths(const QStringList& paths)
{
    g_dialogRecentPaths = paths;
}

bool usesPortalFileDialog()
{
    // A sandbox reaches host files only through the desktop portal, and the
    // built-in dialog would show the sandbox filesystem instead. The override is
    // a test/environment hook.
    return qEnvironmentVariableIsSet("FLATPAK_ID") || qEnvironmentVariableIsSet("SNAP")
        || qEnvironmentVariableIsSet("SNAP_NAME")
        || qEnvironmentVariableIsSet("PICTURA_PORTAL_FILE_DIALOG");
}

void installRollingTypeAhead(QComboBox* combo)
{
    if (!combo) {
        return;
    }
    combo->setEditable(false);
    combo->setInsertPolicy(QComboBox::NoInsert);
    combo->installEventFilter(new RollingTypeAhead(combo));
}

namespace {

// A sandbox must use the platform/portal chooser to reach host files; a native
// install uses the built-in dialog so the enriched Places sidebar and the
// rolling type combo stay under our control. `setSidebarUrls` and the rolling
// install are harmless when a native/portal helper is in use (no Qt widgets).
void configureFileDialog(QFileDialog& dialog, const QStringList& filters,
                         QString* selectedFilter)
{
    const bool portal = usesPortalFileDialog();
    dialog.setOption(QFileDialog::DontUseNativeDialog, !portal);
    dialog.setNameFilters(filters);
    if (!portal) {
        // The platform/portal chooser owns its own places; only the built-in
        // dialog can be populated from here.
        dialog.setSidebarUrls(fileDialogPlaces());
    }
    if (selectedFilter && !selectedFilter->isEmpty()) {
        dialog.selectNameFilter(*selectedFilter);
    }
    // Qt names its combo "fileTypeCombo"; fall back to the dialog's only combo
    // if that private name ever changes.
    QComboBox* typeCombo = dialog.findChild<QComboBox*>(QStringLiteral("fileTypeCombo"));
    if (!typeCombo) {
        typeCombo = dialog.findChild<QComboBox*>();
    }
    installRollingTypeAhead(typeCombo);
}

// KWin's "Dialog Parent" effect dims the parent of a modal window. Run the file
// dialog without Qt's modal hint (so the effect does not fire) and block the
// main window's input with an event filter instead: disabling the window would
// disable the child dialog too, and a real modal window is what triggers the
// dim. The portal chooser is a separate host window, so its modality is left
// alone.
// The canvas still takes a middle-button drag and the wheel, so the image can
// be panned (and wheel-zoomed) to look around while a dialog previews on it.
// ponytail: this only dodges the compositor effect; a user with "Dim Inactive"
// enabled would still see the blocked window dimmed on focus loss.
std::function<bool(QKeyEvent*)>& navigationKeys()
{
    static std::function<bool(QKeyEvent*)> handler;
    return handler;
}

class DialogInputBlocker : public QObject
{
public:
    explicit DialogInputBlocker(QWidget* blocked, QObject* parent)
        : QObject(parent)
        , blocked_(blocked)
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        switch (event->type()) {
        case QEvent::Shortcut: {
            // The frame's menu shortcuts stay blocked; a QShortcut the dialog
            // itself owns (its preview zoom keys) still fires.
            auto* shortcut = qobject_cast<QShortcut*>(watched);
            auto* owner = shortcut ? qobject_cast<QWidget*>(shortcut->parent()) : nullptr;
            return !owner || !blocked_ || owner->window() == blocked_;
        }
        case QEvent::KeyPress:
        case QEvent::KeyRelease:
        case QEvent::ShortcutOverride:
        case QEvent::MouseButtonPress:
        case QEvent::MouseButtonRelease:
        case QEvent::MouseButtonDblClick:
        case QEvent::MouseMove:
        case QEvent::Wheel:
        case QEvent::ContextMenu:
        case QEvent::TouchBegin:
        case QEvent::TouchUpdate:
        case QEvent::TouchEnd:
            break;
        default:
            return QObject::eventFilter(watched, event);
        }
        auto* widget = qobject_cast<QWidget*>(watched);
        if (!widget || !blocked_) {
            return QObject::eventFilter(watched, event);
        }
        // A key the dialog's own shortcuts did not take may be a canvas zoom.
        if (event->type() == QEvent::KeyPress && widget->window() != blocked_
            && navigationKeys() && navigationKeys()(static_cast<QKeyEvent*>(event))) {
            return true;
        }
        if (widget->window() != blocked_ || isCanvasNavigation(widget, event)) {
            return QObject::eventFilter(watched, event);
        }
        // Swallow only input addressed to the blocked top-level window; the
        // dialog and its popups are separate windows and pass through.
        return true;
    }

private:
    // A middle-button press / drag / release or a wheel turn over a canvas.
    static bool isCanvasNavigation(QWidget* widget, QEvent* event)
    {
        bool navigation = event->type() == QEvent::Wheel;
        if (auto* mouse = dynamic_cast<QMouseEvent*>(event)) {
            navigation = event->type() != QEvent::MouseButtonDblClick
                         && (mouse->button() == Qt::MiddleButton
                             || (event->type() == QEvent::MouseMove
                                 && mouse->buttons() == Qt::MiddleButton));
        }
        for (QWidget* w = widget; navigation && w; w = w->parentWidget()) {
            if (qobject_cast<ImageView*>(w)) {
                return true;
            }
        }
        return false;
    }

    QPointer<QWidget> blocked_;
};

int runFileDialog(QFileDialog& dialog, QWidget* parent)
{
    if (usesPortalFileDialog()) {
        return dialog.exec();
    }
    return runDialog(dialog, parent);
}

} // namespace

void setDialogNavigationKeys(std::function<bool(QKeyEvent*)> handler)
{
    navigationKeys() = std::move(handler);
}

int runDialog(QDialog& dialog, QWidget* parent)
{
    if (!parent) {
        return dialog.exec();
    }
    QPointer<QWidget> blocked = parent->window();
    DialogInputBlocker blocker(blocked, qApp);
    qApp->installEventFilter(&blocker);
    dialog.setWindowModality(Qt::NonModal);
    dialog.show();
    QEventLoop loop;
    QObject::connect(&dialog, &QDialog::finished, &loop, &QEventLoop::quit);
    loop.exec();
    qApp->removeEventFilter(&blocker);
    return dialog.result();
}

void addZoomShortcuts(QWidget* dialog, QAbstractButton* zoomIn, QAbstractButton* zoomOut)
{
    auto* in = new QShortcut(dialog);
    in->setKeys({QKeySequence(QStringLiteral("Ctrl++")), QKeySequence(QStringLiteral("Ctrl+=")),
                 QKeySequence(QStringLiteral("Ctrl+Shift+=")),
                 QKeySequence(QStringLiteral("Ctrl+Shift++"))});
    QObject::connect(in, &QShortcut::activated, zoomIn, &QAbstractButton::click);
    // The keymap can offer one press as several of these keys; all are ours.
    QObject::connect(in, &QShortcut::activatedAmbiguously, zoomIn, &QAbstractButton::click);
    auto* out = new QShortcut(QKeySequence(QStringLiteral("Ctrl+-")), dialog);
    QObject::connect(out, &QShortcut::activated, zoomOut, &QAbstractButton::click);
}

QString dialogDirectoryFromStored(const QString& stored)
{
    if (stored.isEmpty()) {
        return QString();
    }
    const QUrl url(stored);
    const QString path = url.isLocalFile() ? url.toLocalFile() : stored;
    return QFileInfo(path).isDir() ? path : QString();
}

QString documentDirectory(const QString& documentPath)
{
    // Guard the empty path: QFileInfo("").absolutePath() is the working dir.
    return documentPath.isEmpty() ? QString() : QFileInfo(documentPath).absolutePath();
}

QString rememberedFileDialogDirectory()
{
    // Qt persists the dialog's last folder itself, but reconstructs it with
    // QUrl::fromLocalFile (it was saved as a file:// URL), which mangles the
    // path and resets an empty-dir dialog to the working directory on restart.
    // Read the same Qt-owned key with the correct conversion.
    // ponytail: depends on Qt's key name; switch to an app-owned key if it moves.
    QSettings settings(QSettings::UserScope, QStringLiteral("QtProject"));
    return dialogDirectoryFromStored(
        settings.value(QStringLiteral("FileDialog/lastVisited")).toString());
}

QString defaultFileDialogDirectory()
{
    const QString remembered = rememberedFileDialogDirectory();
    // First run, or the remembered folder was moved/deleted: start at home.
    return remembered.isEmpty() ? QDir::homePath() : remembered;
}

QString getSaveFileName(QWidget* parent, const QString& caption, const QString& dir,
                        const QStringList& filters, QString* selectedFilter)
{
    QFileDialog dialog(parent, caption,
                       dir.isEmpty() ? defaultFileDialogDirectory() : dir);
    dialog.setAcceptMode(QFileDialog::AcceptSave);
    dialog.setFileMode(QFileDialog::AnyFile);
    configureFileDialog(dialog, filters, selectedFilter);
    if (runFileDialog(dialog, parent) != QDialog::Accepted) {
        return QString();
    }
    if (selectedFilter) {
        *selectedFilter = dialog.selectedNameFilter();
    }
    return dialog.selectedFiles().value(0);
}

QString getOpenFileName(QWidget* parent, const QString& caption, const QString& dir,
                        const QStringList& filters, QString* selectedFilter)
{
    QFileDialog dialog(parent, caption,
                       dir.isEmpty() ? defaultFileDialogDirectory() : dir);
    dialog.setAcceptMode(QFileDialog::AcceptOpen);
    dialog.setFileMode(QFileDialog::ExistingFile);
    configureFileDialog(dialog, filters, selectedFilter);
    if (runFileDialog(dialog, parent) != QDialog::Accepted) {
        return QString();
    }
    if (selectedFilter) {
        *selectedFilter = dialog.selectedNameFilter();
    }
    return dialog.selectedFiles().value(0);
}

} // namespace pictura
