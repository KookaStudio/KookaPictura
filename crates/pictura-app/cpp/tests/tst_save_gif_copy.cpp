// GIF Save As and File > Save a Copy: the format list, the Indexed Color
// hand-off, snapshot encoding, and the non-mutating copy path.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QJsonObject>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtGui/QImageReader>
#include <QtGui/QImageWriter>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStatusBar>

#if defined(Q_OS_UNIX)
#include <unistd.h>
#endif

#include <functional>
#include <memory>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/paint_bridge.h"
#include "canvas_scrollbars.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "document_registry.h"
#include "frame.h"
#include "image_view.h"
#include "import_save_coordinator.h"
#include "indexed_color_dialog.h"
#include "qt_test_support.h"

namespace {

pictura::DocumentRegistry::Hooks testHooks(QWidget* shell,
                                           pictura::ImportSaveCoordinator** coordinator)
{
    pictura::DocumentRegistry::Hooks hooks;
    hooks.shellWidget = [shell]() -> QWidget* { return shell; };
    hooks.openDocumentsAsTabs = []() { return true; };
    hooks.floatingDocumentDocking = []() { return true; };
    hooks.fullScreen = []() { return false; };
    hooks.requestRefresh = []() {};
    hooks.requestPanelRefresh = []() {};
    hooks.stopPanelRefreshTimer = []() {};
    hooks.activateShellWindow = []() {};
    hooks.raisePanelOverlays = []() {};
    hooks.closeWindowRequested = [](pictura::DocWindow*) {};
    hooks.setupCanvas = [shell](pictura::DocWindow& window) {
        window.canvas = new pictura::ImageView(shell);
        window.host = new pictura::CanvasScrollBars(shell);
        window.host->setView(window.canvas);
        pictura::PictureView* view = window.doc->view;
        if (view->has_document()) {
            window.canvas->setDocument(QSize(view->document_width(),
                                            view->document_height()));
        } else {
            window.canvas->replaceImage(view->image());
        }
    };
    hooks.wireCanvas = [](pictura::DocWindow&) {};
    hooks.wireDocument = [](pictura::DocEntry&) {};
    hooks.applyViewSettings = [](pictura::PictureView*) {};
    hooks.wireOpenSignals = [coordinator](pictura::PictureView* view) {
        if (*coordinator) {
            (*coordinator)->wireOpenSignals(view);
        }
    };
    hooks.rememberRecent = [coordinator](const QString& path) {
        if (*coordinator) {
            (*coordinator)->rememberRecent(path);
        }
    };
    hooks.forgetToolView = [](pictura::PictureView*) {};
    hooks.forgetCanvas = [](pictura::ImageView*) {};
    hooks.openDelayForTest = []() -> quint64 { return 0; };
    hooks.loadingPageColor = []() { return QColor(Qt::white); };
    hooks.scrollAllWindows = []() { return false; };
    hooks.toolCanvas = []() -> pictura::ImageView* { return nullptr; };
    return hooks;
}

bool gifWriterAvailable()
{
    return QImageWriter::supportedImageFormats().contains("gif");
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool writeImage(const QString& path, const QImage& image, const char* format = nullptr)
{
    return format ? image.save(path, format) : image.save(path);
}

pictura::SaveFileNameProvider saveProviderFor(const QString& path)
{
    return [path](QWidget*, const QString&, const QString&, const QStringList& filters,
                  QString* selected) {
        if (selected) {
            const QString suffix = QFileInfo(path).suffix();
            if (!suffix.isEmpty()) {
                const QString pattern = QStringLiteral("*.") + suffix;
                for (const QString& filter : filters) {
                    if (filter.contains(pattern, Qt::CaseInsensitive)) {
                        *selected = filter;
                        break;
                    }
                }
            }
        }
        return path;
    };
}

void requireGif(const QString& file, const QSize& size)
{
    QCOMPARE(fileBytes(file).left(6), QByteArrayLiteral("GIF89a"));
    const QImage image = QImageReader(file).read();
    QVERIFY(!image.isNull());
    QCOMPARE(image.size(), size);
}

class ScopedDirectoryPermissions {
public:
    explicit ScopedDirectoryPermissions(QString dir)
        : dir_(std::move(dir))
    {
    }

    ~ScopedDirectoryPermissions()
    {
        QFile::setPermissions(dir_, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                       | QFileDevice::ExeOwner);
    }

private:
    QString dir_;
};

} // namespace

class SaveGifCopyTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void gifFilterOffered();
    void saveRecordedGifPathRewritesGif();
    void typedGifSuffixWritesGif();
    void saveUntitledPromptsForPath();
    void gifImportPreselectsGif();
    void gifWriteFailurePreservesPreviousFile();
    void gifDimensionLimitRefused();
    void rgbGifSaveShowsDialog();
    void cancelledGifSaveWritesNothing();
    void indexedGifSaveSkipsDialog();
    void gifPreviewRestoresAfterDialog();
    void gifPreviewUsesSrgbSnapshot();
    void gifSettingsRemembered();
    void gifExactFallback();
    void indexedSourcePalettePreserved();
    void editedIndexedUsesComposite();
    void quantizedGifMatchesDocumentSize();
    void gifFrom16BitKeepsDepth();
    void gifFrom32BitKeepsDepth();
    void gifFromCmykKeepsMode();
    void layeredGifWarnsAndFlattens();
    void saveCopyEnabledWithDocument();
    void saveCopyDialogMatchesSaveAs();
    void saveCopyCancelWritesNothing();
    void saveCopyProfileDialog();
    void saveCopyNoRecorder();
    void saveCopyRasterLeavesDocument();
    void saveCopyPsdLeavesDocument();
    void saveCopyPdfLeavesDocument();
    void saveCopyUntitledStaysUntitled();
    void saveCopyGifLeavesRgb();
    void saveCopyFailurePreservesPreviousFile();

private:
    pictura::PictureView* newDocument(const QString& name, int width = 8, int height = 8,
                                      const QColor& color = Qt::white);
    pictura::PictureView* openImage(const QImage& image);
    QString path(const QString& name) const { return dir_.filePath(name); }
    void saveGif(const QString& file) { QVERIFY(window_->saveActiveAsWithOptions(file)); }
    QString gifFilter() const { return QStringLiteral("CompuServe GIF (*.GIF)"); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void SaveGifCopyTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

void SaveGifCopyTest::init()
{
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
    pictura::setSaveFileNameProviderForTest(nullptr);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    window_->setProfileDialogForTest([](bool, bool&) { return true; });
    window_->setFeatureWarningForTest([] { return true; });
    window_->setGifDialogForTest([](pictura::PictureView*, pictura::SaveFormatOptions&) {
        return true;
    });
    window_->setGifDialogDriverForTest(nullptr);
}

void SaveGifCopyTest::cleanup()
{
    pictura::setSaveFileNameProviderForTest(nullptr);
}

pictura::PictureView* SaveGifCopyTest::newDocument(const QString& name, int width, int height,
                                                   const QColor& color)
{
    if (!window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, color.rgba(), -1, QStringLiteral("normal"), 100, false);
    return view;
}

pictura::PictureView* SaveGifCopyTest::openImage(const QImage& image)
{
    const QString file = path(QStringLiteral("source.png"));
    if (!writeImage(file, image)) {
        return nullptr;
    }
    if (!window_->openDocumentAtPath(file)) {
        return nullptr;
    }
    return window_->activeView();
}

void SaveGifCopyTest::gifFilterOffered()
{
    const QStringList& filters = pictura::saveFileFilters();
    const int index = filters.indexOf(gifFilter());
    QVERIFY(index >= 0);
    QCOMPARE(pictura::defaultSuffixForFilter(index), QStringLiteral(".gif"));
    QCOMPARE(pictura::pathForSaveFilter(path(QStringLiteral("noext")), index, 0),
             path(QStringLiteral("noext.gif")));
    QCOMPARE(pictura::pathForSaveFilter(path(QStringLiteral("cat.jpg")), index, 0),
             path(QStringLiteral("cat.gif")));
}

void SaveGifCopyTest::saveRecordedGifPathRewritesGif()
{
    QVERIFY(newDocument(QStringLiteral("Save")));
    const QString file = path(QStringLiteral("photo.gif"));
    saveGif(file);
    QCOMPARE(fileBytes(file).left(6), QByteArrayLiteral("GIF89a"));
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(10, 120, 220).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    QVERIFY(window_->saveActive());
    QCOMPARE(fileBytes(file).left(6), QByteArrayLiteral("GIF89a"));
}

void SaveGifCopyTest::typedGifSuffixWritesGif()
{
    QVERIFY(newDocument(QStringLiteral("Typed")));
    const QString file = path(QStringLiteral("cat.gif"));
    saveGif(file);
    QCOMPARE(fileBytes(file).left(6), QByteArrayLiteral("GIF89a"));
}

void SaveGifCopyTest::saveUntitledPromptsForPath()
{
    QVERIFY(newDocument(QStringLiteral("Untitled save")));
    const QString file = path(QStringLiteral("save-untitled.png"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(file));
    QVERIFY(window_->saveActive());
    QVERIFY(QFileInfo::exists(file));
    QCOMPARE(window_->documentPath(0), file);
}

void SaveGifCopyTest::gifImportPreselectsGif()
{
    if (!gifWriterAvailable()) {
        QSKIP("the Qt build writes no GIF");
    }
    QImage image(8, 8, QImage::Format_RGBA8888);
    image.fill(QColor(10, 20, 30));
    const QString file = path(QStringLiteral("import.gif"));
    QVERIFY(writeImage(file, image, "GIF"));
    QVERIFY(window_->openDocumentAtPath(file));
    QCOMPARE(pictura::defaultSaveFilterIndex(window_->activeView()),
             pictura::saveFileFilters().indexOf(gifFilter()));
}

void SaveGifCopyTest::gifWriteFailurePreservesPreviousFile()
{
    QVERIFY(newDocument(QStringLiteral("Failure")));
    const QString directory = path(QStringLiteral("locked"));
    QVERIFY(QDir().mkpath(directory));
    ScopedDirectoryPermissions restore(directory);
    const QString file = directory + QStringLiteral("/photo.gif");
    QFile existing(file);
    QVERIFY(existing.open(QIODevice::WriteOnly));
    existing.write("old bytes");
    existing.close();
#if defined(Q_OS_UNIX)
    if (::geteuid() == 0) {
        QSKIP("root ignores directory permissions");
    }
#endif
    QVERIFY(QFile::setPermissions(
        directory, QFileDevice::ReadOwner | QFileDevice::ExeOwner));
    QVERIFY(!window_->saveActiveAsWithOptions(file));
    QCOMPARE(fileBytes(file), QByteArrayLiteral("old bytes"));
}

void SaveGifCopyTest::gifDimensionLimitRefused()
{
    if (!newDocument(QStringLiteral("Wide"), 70000, 1)) {
        QSKIP("the document limit refuses a 70,000-pixel canvas");
    }
    const QString file = path(QStringLiteral("wide.gif"));
    QVERIFY(!window_->saveActiveAsWithOptions(file));
    QVERIFY(!QFileInfo::exists(file));
}

void SaveGifCopyTest::rgbGifSaveShowsDialog()
{
    QVERIFY(newDocument(QStringLiteral("Dialog")));
    pictura::SaveFormatKind seen = pictura::SaveFormatKind::None;
    window_->setSaveFormatDialogForTest(
        [&seen](pictura::SaveFormatKind kind, pictura::SaveFormatOptions&) {
            seen = kind;
            return true;
        });
    saveGif(path(QStringLiteral("dialog.gif")));
    QCOMPARE(seen, pictura::SaveFormatKind::Gif);
}

void SaveGifCopyTest::cancelledGifSaveWritesNothing()
{
    QVERIFY(newDocument(QStringLiteral("Cancel")));
    const int history = window_->activeView()->history_count();
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return false; });
    const QString file = path(QStringLiteral("cancel.gif"));
    QVERIFY(!window_->saveActiveAsWithOptions(file));
    QVERIFY(!QFileInfo::exists(file));
    QCOMPARE(window_->activeView()->history_count(), history);
    QCOMPARE(pictura::image_mode(*window_->activeView()), QStringLiteral("rgb"));
}

void SaveGifCopyTest::indexedGifSaveSkipsDialog()
{
    QVERIFY(newDocument(QStringLiteral("Indexed")));
    QVERIFY(pictura::convert_to_indexed(*window_->activeView(), 1, 256, 0, 0));
    int calls = 0;
    window_->setSaveFormatDialogForTest(
        [&calls](pictura::SaveFormatKind, pictura::SaveFormatOptions&) {
            ++calls;
            return true;
        });
    saveGif(path(QStringLiteral("indexed.gif")));
    QCOMPARE(calls, 0);
}

void SaveGifCopyTest::gifPreviewRestoresAfterDialog()
{
    QVERIFY(newDocument(QStringLiteral("Preview")));
    pictura::PictureView* view = window_->activeView();
    const int history = view->history_count();
    window_->setGifDialogForTest(nullptr);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind kind, pictura::SaveFormatOptions&) {
            return kind == pictura::SaveFormatKind::Gif;
        });

    const auto drive = [](bool accept) {
        return [accept](pictura::IndexedColorDialog& dialog) {
            auto* preview =
                dialog.findChild<QCheckBox*>(QStringLiteral("indexedPreview"));
            if (!preview) {
                return false;
            }
            preview->setChecked(false);
            preview->setChecked(true);
            return accept;
        };
    };

    window_->setGifDialogDriverForTest(drive(true));
    QVERIFY(window_->saveActiveAsWithOptions(path(QStringLiteral("preview-accept.gif"))));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->history_count(), history);
    QVERIFY(QFileInfo::exists(path(QStringLiteral("preview-accept.gif"))));

    window_->setGifDialogDriverForTest(drive(false));
    QVERIFY(!window_->saveActiveAsWithOptions(path(QStringLiteral("preview-cancel.gif"))));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->history_count(), history);
    QVERIFY(!QFileInfo::exists(path(QStringLiteral("preview-cancel.gif"))));

    window_->setGifDialogDriverForTest(nullptr);
}

void SaveGifCopyTest::gifPreviewUsesSrgbSnapshot()
{
    QVERIFY(newDocument(QStringLiteral("Preview sRGB"), 8, 8, QColor(200, 40, 20)));
    pictura::PictureView* view = window_->activeView();
    if (!view->assign_profile(1)) {
        QSKIP("the Adobe RGB profile is unavailable");
    }
    const int history = view->history_count();
    window_->setGifDialogForTest(nullptr);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind kind, pictura::SaveFormatOptions&) {
            return kind == pictura::SaveFormatKind::Gif;
        });
    QColor previewColor;
    const auto drive = [&previewColor, view](pictura::IndexedColorDialog& dialog) {
        auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("indexedPreview"));
        if (!preview) {
            return false;
        }
        preview->setChecked(false);
        preview->setChecked(true);
        previewColor = view->image().pixelColor(4, 4);
        return true;
    };
    window_->setGifDialogDriverForTest(drive);
    const QString file = path(QStringLiteral("profile-preview.gif"));
    QVERIFY(window_->saveActiveAsWithOptions(file));
    const QImage saved = QImageReader(file).read();
    QVERIFY(!saved.isNull());
    const QColor savedColor = saved.pixelColor(4, 4);
    QVERIFY2(qAbs(previewColor.red() - savedColor.red()) <= 2
                 && qAbs(previewColor.green() - savedColor.green()) <= 2
                 && qAbs(previewColor.blue() - savedColor.blue()) <= 2,
             qPrintable(QStringLiteral("preview %1, saved %2")
                            .arg(previewColor.name(), savedColor.name())));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->history_count(), history);
    window_->setGifDialogDriverForTest(nullptr);
}

void SaveGifCopyTest::gifSettingsRemembered()
{
    QVERIFY(newDocument(QStringLiteral("Remember")));
    pictura::SaveFormatOptions seen;
    int calls = 0;
    window_->setSaveFormatDialogForTest(
        [&seen, &calls](pictura::SaveFormatKind kind, pictura::SaveFormatOptions& options) {
            if (kind == pictura::SaveFormatKind::Gif) {
                if (calls++ == 0) {
                    options.gifPalette = 1;
                    options.gifColors = 216;
                    options.gifDither = 3;
                    options.gifAmount = 40;
                } else {
                    seen = options;
                }
            }
            return true;
        });
    saveGif(path(QStringLiteral("remember-1.gif")));
    saveGif(path(QStringLiteral("remember-2.gif")));
    QCOMPARE(seen.gifPalette, 1);
    QCOMPARE(seen.gifColors, 216);
    QCOMPARE(seen.gifDither, 3);
    QCOMPARE(seen.gifAmount, 40);
}

void SaveGifCopyTest::gifExactFallback()
{
    pictura::IndexedColorOptions initial;
    initial.palette = 0;
    pictura::IndexedColorDialog unavailable(false, initial);
    QVERIFY(unavailable.palette() != 0);
    pictura::IndexedColorDialog available(true, initial);
    QCOMPARE(available.palette(), 0);
}

void SaveGifCopyTest::indexedSourcePalettePreserved()
{
    QImage image(16, 16, QImage::Format_RGBA8888);
    for (int y = 0; y < 16; ++y) {
        for (int x = 0; x < 16; ++x) {
            const int color = (y / 4) * 4 + x / 4;
            image.setPixelColor(x, y,
                                QColor((color * 37) % 256, (color * 71 + 20) % 256,
                                       (color * 113 + 40) % 256));
        }
    }
    pictura::PictureView* view = openImage(image);
    QVERIFY(view);

    // Remember two colors for the GIF write; an unchanged Indexed source must
    // still use its own 16-color palette and index order.
    bool trained = false;
    window_->setSaveFormatDialogForTest(
        [&trained](pictura::SaveFormatKind kind, pictura::SaveFormatOptions& options) {
            if (kind == pictura::SaveFormatKind::Gif && !trained) {
                options.gifPalette = 2;
                options.gifColors = 2;
                options.gifDither = 0;
                trained = true;
            }
            return true;
        });
    saveGif(path(QStringLiteral("palette-training.gif")));
    QVERIFY(trained);
    QVERIFY(pictura::convert_to_indexed(*view, 3, 16, 0, 0));

    const QString file = path(QStringLiteral("palette.gif"));
    saveGif(file);
    const QByteArray bytes = fileBytes(file);
    QVERIFY(bytes.size() > 13);
    const int packed = quint8(bytes.at(10));
    QVERIFY2(packed & 0x80, "the GIF has no global colour table");
    QCOMPARE(1 << ((packed & 0x07) + 1), 16);
}

void SaveGifCopyTest::editedIndexedUsesComposite()
{
    QVERIFY(newDocument(QStringLiteral("Edited"), 8, 8, Qt::white));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::convert_to_indexed(*view, 1, 256, 0, 0));
    view->select_all();
    pictura::edit_fill(*view, QColor(220, 20, 20).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    const QString file = path(QStringLiteral("edited.gif"));
    saveGif(file);
    const QImage image = QImageReader(file).read();
    QVERIFY(!image.isNull());
    const QColor pixel = image.pixelColor(4, 4);
    QVERIFY2(pixel.red() > 150 && pixel.green() < 100, qPrintable(pixel.name()));
}

void SaveGifCopyTest::quantizedGifMatchesDocumentSize()
{
    QImage gradient(16, 16, QImage::Format_RGBA8888);
    for (int y = 0; y < 16; ++y) {
        for (int x = 0; x < 16; ++x) {
            gradient.setPixelColor(x, y, QColor(x * 15, y * 15, (x * y) % 256));
        }
    }
    QVERIFY(openImage(gradient));
    const QString file = path(QStringLiteral("quantized.gif"));
    saveGif(file);
    const QImage image = QImageReader(file).read();
    QVERIFY(!image.isNull());
    QCOMPARE(image.size(), QSize(16, 16));
    QVERIFY(image.colorCount() <= 256);
}

void SaveGifCopyTest::gifFrom16BitKeepsDepth()
{
    QVERIFY(newDocument(QStringLiteral("16-bit")));
    QVERIFY(pictura::convert_image_depth(*window_->activeView(), 16));
    const QString file = path(QStringLiteral("16bit.gif"));
    saveGif(file);
    requireGif(file, QSize(8, 8));
    QCOMPARE(pictura::image_depth_bits(*window_->activeView()), 16);
}

void SaveGifCopyTest::gifFrom32BitKeepsDepth()
{
    QVERIFY(newDocument(QStringLiteral("32-bit")));
    QVERIFY(pictura::convert_image_depth(*window_->activeView(), 16));
    QVERIFY(pictura::convert_image_depth(*window_->activeView(), 32));
    const QString file = path(QStringLiteral("32bit.gif"));
    saveGif(file);
    requireGif(file, QSize(8, 8));
    QCOMPARE(pictura::image_depth_bits(*window_->activeView()), 32);
}

void SaveGifCopyTest::gifFromCmykKeepsMode()
{
    QVERIFY(newDocument(QStringLiteral("CMYK")));
    QVERIFY(pictura::convert_image_mode(*window_->activeView(), QStringLiteral("cmyk")));
    const QString file = path(QStringLiteral("cmyk.gif"));
    saveGif(file);
    requireGif(file, QSize(8, 8));
    QCOMPARE(pictura::image_mode(*window_->activeView()), QStringLiteral("cmyk"));
}

void SaveGifCopyTest::layeredGifWarnsAndFlattens()
{
    QVERIFY(newDocument(QStringLiteral("Layered")));
    int warnings = 0;
    window_->setFeatureWarningForTest([&warnings] {
        ++warnings;
        return true;
    });

    saveGif(path(QStringLiteral("flat.gif")));
    QCOMPARE(warnings, 0);

    QVERIFY(window_->activeView()->add_layer(0) >= 0);
    saveGif(path(QStringLiteral("layered.gif")));
    QCOMPARE(warnings, 1);
    requireGif(path(QStringLiteral("layered.gif")), QSize(8, 8));
}

void SaveGifCopyTest::saveCopyEnabledWithDocument()
{
    QAction* action = window_->registry()->action(QLatin1String(pictura::command_ids::FileSaveCopy));
    QVERIFY(action);
    QVERIFY(!action->isEnabled());
    QVERIFY(newDocument(QStringLiteral("Copy")));
    QVERIFY(action->isEnabled());
}

void SaveGifCopyTest::saveCopyDialogMatchesSaveAs()
{
    QImage image(8, 8, QImage::Format_RGBA8888);
    image.fill(Qt::white);
    QVERIFY(openImage(image));
    QStringList seenFilters;
    QString seenSelected;
    pictura::setSaveFileNameProviderForTest(
        [&seenFilters, &seenSelected](QWidget*, const QString&, const QString&,
                                      const QStringList& filters, QString* selected) {
            seenFilters = filters;
            if (selected) {
                seenSelected = *selected;
            }
            return QString();
        });
    QVERIFY(!window_->saveCopyWithDialog());
    QCOMPARE(seenFilters, pictura::saveFileFilters());
    QCOMPARE(seenSelected, QStringLiteral("PNG (*.PNG)"));
}

void SaveGifCopyTest::saveCopyCancelWritesNothing()
{
    QVERIFY(newDocument(QStringLiteral("Copy cancel")));
    const QString file = path(QStringLiteral("cancelled-copy.png"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(file));
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return false; });
    QVERIFY(!window_->saveCopyWithDialog());
    QVERIFY(!QFileInfo::exists(file));
    QVERIFY(window_->documentPath(0).isEmpty());
}

void SaveGifCopyTest::saveCopyProfileDialog()
{
    QVERIFY(newDocument(QStringLiteral("Copy profile")));
    const QString file = path(QStringLiteral("profile.psd"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(file));
    window_->setProfileDialogForTest([](bool, bool&) { return false; });
    QVERIFY(!window_->saveCopyWithDialog());
    QVERIFY(!QFileInfo::exists(file));
    QVERIFY(window_->documentPath(0).isEmpty());
}

void SaveGifCopyTest::saveCopyNoRecorder()
{
    QWidget shell;
    pictura::ImportSaveCoordinator* coordinator = nullptr;
    auto registry = std::make_unique<pictura::DocumentRegistry>(testHooks(&shell, &coordinator),
                                                                &shell);
    int recorded = 0;
    pictura::ImportSaveCoordinator::Hooks hooks;
    hooks.applyColorSettings = [](pictura::PictureView*) {};
    hooks.requestRefresh = []() {};
    hooks.updateWindowTitle = []() {};
    hooks.showStatus = [](const QString&, int) {};
    hooks.recordCommand = [&recorded](const QString&, const QJsonObject&) { ++recorded; };
    auto save = std::make_unique<pictura::ImportSaveCoordinator>(*registry, &shell, hooks);
    coordinator = save.get();
    save->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    auto* view = new pictura::PictureView(&shell);
    QVERIFY(pictura::document_api::newDocument(*view, 8, 8, QStringLiteral("rgb"), 8,
                                               QStringLiteral("white")));
    QVERIFY(registry->addDocument(view, QString(), true) >= 0);
    QVERIFY(save->saveCopyAsWithOptions(path(QStringLiteral("no-recorder.png"))));
    QCOMPARE(recorded, 0);
}

void SaveGifCopyTest::saveCopyRasterLeavesDocument()
{
    QVERIFY(newDocument(QStringLiteral("Copy raster")));
    pictura::PictureView* view = window_->activeView();
    const QString original = path(QStringLiteral("a.psd"));
    QVERIFY(window_->saveActiveAs(original));
    view->select_all();
    pictura::edit_fill(*view, QColor(200, 10, 10).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    const int history = view->history_count();
    const QString copy = path(QStringLiteral("b.png"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(copy));
    QVERIFY(window_->saveCopyWithDialog());
    QVERIFY(QFileInfo::exists(copy));
    QCOMPARE(window_->documentPath(0), original);
    QVERIFY(pictura::document_api::isDirty(*view));
    QCOMPARE(view->history_count(), history);
}

void SaveGifCopyTest::saveCopyPsdLeavesDocument()
{
    QVERIFY(newDocument(QStringLiteral("Copy PSD")));
    pictura::PictureView* view = window_->activeView();
    const QString original = path(QStringLiteral("doc.psd"));
    QVERIFY(window_->saveActiveAs(original));
    const QString copy = path(QStringLiteral("backup.psd"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(copy));
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    window_->setProfileDialogForTest([](bool, bool&) { return true; });
    QVERIFY(window_->saveCopyWithDialog());
    QVERIFY(QFileInfo::exists(copy));
    QCOMPARE(window_->documentPath(0), original);
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
}

void SaveGifCopyTest::saveCopyPdfLeavesDocument()
{
    QVERIFY(newDocument(QStringLiteral("Copy PDF")));
    const QString original = path(QStringLiteral("doc.psd"));
    QVERIFY(window_->saveActiveAs(original));
    const QString copy = path(QStringLiteral("print.pdf"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(copy));
    window_->setPdfDialogDriverForTest([](QDialog& dialog) {
        dialog.accept();
        return true;
    });
    QVERIFY(window_->saveCopyWithDialog());
    QVERIFY(QFileInfo::exists(copy));
    QCOMPARE(window_->documentPath(0), original);
}

void SaveGifCopyTest::saveCopyUntitledStaysUntitled()
{
    QVERIFY(newDocument(QStringLiteral("Untitled")));
    const QString copy = path(QStringLiteral("untitled.png"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(copy));
    QVERIFY(window_->saveCopyWithDialog());
    QVERIFY(QFileInfo::exists(copy));
    QVERIFY(window_->documentPath(0).isEmpty());
}

void SaveGifCopyTest::saveCopyGifLeavesRgb()
{
    QVERIFY(newDocument(QStringLiteral("Copy GIF")));
    pictura::PictureView* view = window_->activeView();
    const int history = view->history_count();
    const QString copy = path(QStringLiteral("web.gif"));
    pictura::setSaveFileNameProviderForTest(saveProviderFor(copy));
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind kind, pictura::SaveFormatOptions&) {
            return kind == pictura::SaveFormatKind::Gif;
        });
    QVERIFY(window_->saveCopyWithDialog());
    QCOMPARE(fileBytes(copy).left(6), QByteArrayLiteral("GIF89a"));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QVERIFY(window_->documentPath(0).isEmpty());
    QCOMPARE(view->history_count(), history);
}

void SaveGifCopyTest::saveCopyFailurePreservesPreviousFile()
{
    QVERIFY(newDocument(QStringLiteral("Copy failure")));
    const QString directory = path(QStringLiteral("locked-copy"));
    QVERIFY(QDir().mkpath(directory));
    ScopedDirectoryPermissions restore(directory);
    const QString file = directory + QStringLiteral("/copy.png");
    QFile existing(file);
    QVERIFY(existing.open(QIODevice::WriteOnly));
    existing.write("old bytes");
    existing.close();
#if defined(Q_OS_UNIX)
    if (::geteuid() == 0) {
        QSKIP("root ignores directory permissions");
    }
#endif
    pictura::setSaveFileNameProviderForTest(saveProviderFor(file));
    QVERIFY(QFile::setPermissions(
        directory, QFileDevice::ReadOwner | QFileDevice::ExeOwner));
    QVERIFY(!window_->saveCopyWithDialog());
    QCOMPARE(fileBytes(file), QByteArrayLiteral("old bytes"));
    QVERIFY(!QFileInfo::exists(directory + QStringLiteral("/copy.psd")));
    QVERIFY2(window_->statusBar()->currentMessage().contains(QStringLiteral("copy"),
                                                             Qt::CaseInsensitive),
             qPrintable(window_->statusBar()->currentMessage()));
}

QTEST_MAIN(SaveGifCopyTest)
#include "tst_save_gif_copy.moc"
