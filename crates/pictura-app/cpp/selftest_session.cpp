#include "selftest_session.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "export_as_dialog.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "panels/panel_column.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonValue>
#include <QtCore/QMetaObject>
#include <QtCore/QSettings>
#include <QtCore/QUrl>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>

namespace pictura {

namespace {

void pump(int count)
{
    for (int i = 0; i < count; ++i) {
        QCoreApplication::processEvents();
    }
}

QString writeSolidImage(const QString& path, const QColor& color)
{
    QImage image(4, 4, QImage::Format_RGBA8888);
    image.fill(color);
    return image.save(path, "PNG") ? path : QString();
}

} // namespace

int runSessionChecks(pictura::PicturaMainWindow& frame)
{
        // lpr_quit_save (317): the application's quit signal (what File > Exit
        // and Ctrl+Q reach) runs the session save even though `closeEvent` never
        // fires. A sentinel `layoutRevision` is overwritten by the live save.
        {
            pictura::SessionState sentinel = pictura::loadSession();
            sentinel.layoutRevision = 12345;
            pictura::saveSession(sentinel);
            if (qApp) {
                // `aboutToQuit` carries a `QPrivateSignal`, so emit it through
                // the meta-object exactly as `QCoreApplication::quit()` does.
                QMetaObject::invokeMethod(qApp, "aboutToQuit", Qt::DirectConnection);
            }
            const pictura::SessionState after = pictura::loadSession();
            const bool quitSaved =
                after.layoutRevision == PicturaMainWindow::kLayoutRevision
                && after.schemaVersion >= 7;
            ST_BEGIN("lpr_quit_save");
            ST_PASS("lpr_quit_save revision=%d schema=%d", after.layoutRevision,
                    after.schemaVersion);
            if (!quitSaved) {
                return pictura::selfTest().fail(317, "quit-path session save");
            }
        }

        // lpr_width_restart (318): resizing the columns and quitting saves each
        // column's width; re-running the startup restore applies it and each
        // width is measurable after the first layout.
        {
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
            pictura::PanelColumn* primary = frame.panelColumn();
            const bool madeLeft =
                frame.newColumnDropForTest(QStringLiteral("stylesPanel"), QStringLiteral("left"));
            pump(4);
            pictura::PanelColumn* left = frame.columnForPanel(QStringLiteral("stylesPanel"));
            if (left) {
                left->setPreferredWidth(205);
            }
            if (primary) {
                primary->setPreferredWidth(245);
            }
            pump(6);
            const int leftWidth = left ? left->width() : -1;
            const int primaryWidth = primary ? primary->width() : -1;
            frame.saveSession();
            const pictura::SessionState loaded = pictura::loadSession();

            int storedLeft = -1;
            int storedRight = -1;
            for (const QJsonValue& value : loaded.panelColumns) {
                const QJsonObject entry = value.toObject();
                if (entry.value(QStringLiteral("tools")).toBool()) {
                    continue;
                }
                const int width = entry.value(QStringLiteral("width")).toInt(-1);
                if (entry.value(QStringLiteral("side")).toString() == QStringLiteral("left")
                    && storedLeft < 0) {
                    storedLeft = width;
                } else if (entry.value(QStringLiteral("side")).toString()
                               == QStringLiteral("right")
                           && storedRight < 0) {
                    storedRight = width;
                }
            }
            const bool storedOk =
                madeLeft && leftWidth > 0 && primaryWidth > 0 && storedLeft == leftWidth
                && storedRight == primaryWidth;

            frame.applyPanelSessionForTest(loaded);
            pump(8);
            pictura::PanelColumn* restoredLeft =
                frame.columnForPanel(QStringLiteral("stylesPanel"));
            const bool appliedOk = frame.panelColumnCountForTest() == 3 && restoredLeft
                && restoredLeft->width() == leftWidth
                && frame.panelColumn()->width() == primaryWidth;
            ST_BEGIN("lpr_width_restart");
            ST_PASS("lpr_width_restart left=%d/%d right=%d/%d columns=%d", leftWidth,
                    storedLeft, primaryWidth, storedRight, frame.panelColumnCountForTest());
            if (!storedOk || !appliedOk) {
                return pictura::selfTest().fail(318, "column width restart");
            }
        }

        // lpr_v7_width (319): a v6 per-column entry with no width loads the
        // default, except the primary column, which is seeded from the legacy
        // top-level `railWidth`. An iconic store exposes the loaded normal width
        // through `persistedWidth()` without the normal-mode minimum clamping it.
        {
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
            auto writeV6 = [](int railWidth) {
                QFile store(pictura::sessionFilePath());
                if (!store.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
                    return false;
                }
                const QByteArray json =
                    QStringLiteral("{\"schemaVersion\":6,\"panelRailMode\":\"iconic\","
                                   "\"railWidth\":%1,\"panelColumns\":[{\"side\":\"right\","
                                   "\"order\":0,\"groups\":[]}]}")
                        .arg(railWidth)
                        .toUtf8();
                const bool ok = store.write(json) == json.size();
                store.close();
                return ok;
            };

            // The legacy `railWidth` is a normal-mode width, so it must sit above
            // the shared normal-mode floor to be carried through unclamped.
            const int kLegacyWidth = 334;
            const bool seededWritten = writeV6(kLegacyWidth);
            frame.applyPanelSessionForTest(pictura::loadSession());
            pump(8);
            pictura::PanelColumn* primary = frame.panelColumn();
            const bool iconic = primary && primary->railMode();
            const int seededWidth = primary ? primary->persistedWidth() : -1;

            const bool defaultWritten = writeV6(0);
            frame.applyPanelSessionForTest(pictura::loadSession());
            pump(8);
            primary = frame.panelColumn();
            const int defaultWidth = primary ? primary->persistedWidth() : -1;
            const bool v7Ok = seededWritten && defaultWritten && iconic && primary
                && primary->railMode() && seededWidth == kLegacyWidth
                && defaultWidth == pictura::PanelColumn::kDefaultNormalWidth;
            ST_BEGIN("lpr_v7_width");
            ST_PASS("lpr_v7_width seeded=%d default=%d/%d", seededWidth, defaultWidth,
                    pictura::PanelColumn::kDefaultNormalWidth);
            if (!v7Ok) {
                return pictura::selfTest().fail(319, "v6 default width");
            }
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
        }

        // lpr_iconic_flip_width (399): an iconic->normal flip persists a sane
        // normal width, never the icon strip and never an oversized value that
        // would expand the column across the workspace on the next launch.
        {
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
            pictura::PanelColumn* primary = frame.panelColumn();
            if (!primary) {
                return pictura::selfTest().fail(399, "iconic flip fixture");
            }
            primary->setPreferredWidth(245);
            pump(6);
            primary->setRailMode(true);
            const int kRemembered = 320;
            primary->setRestoredWidth(kRemembered);
            primary->setRailMode(false);
            frame.saveSession();

            int storedRight = -1;
            const pictura::SessionState loaded = pictura::loadSession();
            for (const QJsonValue& value : loaded.panelColumns) {
                const QJsonObject entry = value.toObject();
                if (entry.value(QStringLiteral("side")).toString() == QStringLiteral("right")
                    && storedRight < 0) {
                    storedRight = entry.value(QStringLiteral("width")).toInt(-1);
                }
            }
            primary->setRailMode(true);
            primary->setRestoredWidth(1000000);
            const int clamped = primary->persistedWidth();
            frame.applyPanelSessionForTest(loaded);
            pump(8);
            primary = frame.panelColumn();
            const bool restoredOk = primary && !primary->railMode();
            const bool storedOk = storedRight >= pictura::PanelColumn::kMinNormalWidth
                && storedRight <= pictura::PanelColumn::kMaxNormalWidth;
            ST_BEGIN("lpr_iconic_flip_width");
            ST_PASS("lpr_iconic_flip_width stored=%d remembered=%d clamped=%d "
                    "restoredMode=%d width=%d",
                    storedRight, kRemembered, clamped, restoredOk ? 1 : 0,
                    primary ? primary->width() : -1);
            if (!storedOk || !restoredOk
                || clamped != pictura::PanelColumn::kMaxNormalWidth) {
                return pictura::selfTest().fail(399, "iconic->normal width persistence");
            }
            // Leave a sane store behind.
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
            if (pictura::PanelColumn* reset = frame.panelColumn()) {
                reset->setPreferredWidth(245);
            }
            frame.saveSession();
        }

        // lpr_workspace_space (400): with no document open the workspace pane
        // stays in the splitter at its minimum width and keeps the stretch, so
        // the widget columns can never absorb it and the splitter keeps a
        // grabbable handle on both sides; an iconic column keeps its narrow
        // minimum but stays resizable, and shrinking a normal column gives the
        // slack back to the workspace rather than to a hidden pane.
        {
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
            while (frame.documentCount() > 0) {
                frame.closeDocument(0, false);
            }
            pump(6);
            auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
            auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
            const int wsIndex = (cs && tabs) ? cs->indexOf(tabs) : -1;
            const int splitterW = cs ? cs->width() : -1;
            const int wsWidth = tabs ? tabs->width() : -1;
            const bool paneShown = tabs && tabs->isVisible();
            const bool stripHidden =
                tabs && tabs->tabBar() && !tabs->tabBar()->isVisible();
            const bool handleLeft =
                cs && wsIndex <= 0 ? true : (cs->handle(wsIndex - 1) != nullptr);
            const bool handleRight = cs && wsIndex >= 0 && wsIndex < cs->count() - 1
                                     && cs->handle(wsIndex) != nullptr;
            int columnsW = 0;
            for (pictura::PanelColumn* column : frame.panelColumns()) {
                columnsW += column->width();
            }
            const bool reserved = splitterW > 0 && wsWidth >= tabs->minimumWidth()
                                  && columnsW + wsWidth <= splitterW;

            pictura::PanelColumn* primary = frame.panelColumn();
            bool iconicNarrow = false;
            bool wsAbsorbs = false;
            int beforeWs = -1;
            int beforeCol = -1;
            int afterWs = -1;
            int afterCol = -1;
            if (primary) {
                primary->setRailMode(true);
                pump(6);
                iconicNarrow = primary->width() > 0
                               && primary->minimumWidthForTest() <= 60
                               && primary->maximumWidth() > primary->minimumWidthForTest();
                primary->setRailMode(false);
                pump(6);
                beforeWs = tabs->width();
                beforeCol = primary->width();
                primary->setPreferredWidth(beforeCol + 60);
                pump(6);
                afterWs = tabs->width();
                afterCol = primary->width();
                // The workspace yields exactly what the column gains: the handle
                // sits between them, not beside a hidden pane.
                wsAbsorbs = afterCol > beforeCol
                            && (beforeWs - afterWs) == (afterCol - beforeCol);
            }
            ST_BEGIN("lpr_workspace_space");
            ST_PASS("lpr_workspace_space shown=%d strip=%d hl=%d hr=%d ws=%d cols=%d "
                    "split=%d iconic=%d absorbs=%d wsBefore=%d wsAfter=%d "
                    "colBefore=%d colAfter=%d",
                    paneShown ? 1 : 0, stripHidden ? 1 : 0, handleLeft ? 1 : 0,
                    handleRight ? 1 : 0, wsWidth, columnsW, splitterW,
                    iconicNarrow ? 1 : 0, wsAbsorbs ? 1 : 0, beforeWs, afterWs,
                    beforeCol, afterCol);
            if (!(paneShown && stripHidden && handleLeft && handleRight && reserved
                  && iconicNarrow && wsAbsorbs)) {
                return pictura::selfTest().fail(400, "workspace space");
            }
            frame.applyPanelSessionForTest(pictura::SessionState{});
            pump(6);
        }

        // ldt_mode_bits (320): an opened raster's tab reads `base @ 100%
        // (RGB/8)` rather than a generated `Untitled-N` name.
        const QString modePath = QDir::tempPath() + QStringLiteral("/modebits.png");
        {
            const QString written =
                writeSolidImage(modePath, QColor(255, 0, 0, 255));
            const bool opened = !written.isEmpty() && frame.openImagePath(modePath);
            const int index = frame.activeDocumentIndex();
            const QString tabText = frame.documentTabTextForTest(index);
            const bool titleOk =
                opened && tabText == QStringLiteral("modebits.png @ 100% (RGB/8)");
            ST_BEGIN("ldt_mode_bits");
            ST_PASS("ldt_mode_bits opened=%d title=%s", opened ? 1 : 0,
                    qPrintable(tabText));
            if (!titleOk) {
                return pictura::selfTest().fail(320, "tab mode/bits title");
            }
            frame.closeDocument(index, false);
        }
        QFile::remove(modePath);

        // ldt_untitled (321): a document with no name still shows `Untitled-N`.
        {
            const bool made = frame.newDocument(QStringLiteral("Untitled"), 8, 8,
                                                QStringLiteral("rgb"), 8,
                                                QStringLiteral("white"));
            const int index = frame.activeDocumentIndex();
            const QString name = frame.documentName(index);
            const QString tabText = frame.documentTabTextForTest(index);
            const bool untitledOk = made && name.startsWith(QStringLiteral("Untitled-"))
                && tabText.startsWith(name)
                && tabText.contains(QStringLiteral("(RGB/8)"));
            ST_BEGIN("ldt_untitled");
            ST_PASS("ldt_untitled name=%s title=%s", qPrintable(name),
                    qPrintable(tabText));
            if (!untitledOk) {
                return pictura::selfTest().fail(321, "untitled tab name");
            }
            frame.closeDocument(index, false);
        }

        // lim_opaque_background (322): a fully opaque import becomes the locked
        // `Background` with its redundant alpha channel dropped.
        const QString opaquePath =
            QDir::tempPath() + QStringLiteral("/kooka-pictura-opaque.png");
        {
            const QString written =
                writeSolidImage(opaquePath, QColor(20, 120, 220, 255));
            const bool opened = !written.isEmpty() && frame.openImagePath(opaquePath);
            pictura::PictureView* view = frame.activeView();
            const bool opaqueOk = opened && view && pictura::layers_api::rowCount(*view) == 1
                && pictura::layers_api::rowAt(*view, 0).kind == QStringLiteral("background")
                && pictura::layers_api::rowAt(*view, 0).lockBits == 0x05;
            ST_BEGIN("lim_opaque_background");
            ST_PASS("lim_opaque_background opened=%d kind=%s lock=%d", opened ? 1 : 0,
                    view ? qPrintable(pictura::layers_api::rowAt(*view, 0).kind) : "-",
                    view ? pictura::layers_api::rowAt(*view, 0).lockBits : -1);
            if (!opaqueOk) {
                return pictura::selfTest().fail(322, "opaque import background");
            }
            frame.closeDocument(frame.activeDocumentIndex(), false);
        }
        QFile::remove(opaquePath);

        // lim_transparent_layer (323): an import with a non-opaque pixel stays a
        // regular alpha layer (not a locked Background).
        const QString alphaPath =
            QDir::tempPath() + QStringLiteral("/kooka-pictura-alpha.png");
        {
            const QString written =
                writeSolidImage(alphaPath, QColor(0, 160, 0, 128));
            const bool opened = !written.isEmpty() && frame.openImagePath(alphaPath);
            pictura::PictureView* view = frame.activeView();
            const unsigned int pixel = view ? view->sample_argb(0, 0) : 0u;
            const bool alphaOk = opened && view && pictura::layers_api::rowCount(*view) == 1
                && pictura::layers_api::rowAt(*view, 0).kind != QStringLiteral("background")
                && pictura::layers_api::rowAt(*view, 0).lockBits != 0x0F
                && ((pixel >> 24) & 0xffu) != 0xffu;
            ST_BEGIN("lim_transparent_layer");
            ST_PASS("lim_transparent_layer opened=%d kind=%s lock=%d pixel=%08x",
                    opened ? 1 : 0,
                    view ? qPrintable(pictura::layers_api::rowAt(*view, 0).kind) : "-",
                    view ? pictura::layers_api::rowAt(*view, 0).lockBits : -1, pixel);
            if (!alphaOk) {
                return pictura::selfTest().fail(323, "transparent import layer");
            }
            frame.closeDocument(frame.activeDocumentIndex(), false);
        }
        QFile::remove(alphaPath);

        // session_color_policy (462): the incoming-profile policy round-trips
        // through the session store, an out-of-range stored code normalizes to
        // Preserve, and a fresh SessionState defaults to Preserve.
        {
            pictura::SessionState cpsState = pictura::loadSession();
            cpsState.colorPolicy = 1;
            const bool cpsSavedConvert = pictura::saveSession(cpsState);
            const bool cpsRoundConvert = pictura::loadSession().colorPolicy == 1;
            cpsState.colorPolicy = 2;
            const bool cpsSavedOff = pictura::saveSession(cpsState);
            const bool cpsRoundOff = pictura::loadSession().colorPolicy == 2;
            cpsState.colorPolicy = 0;
            pictura::saveSession(cpsState);

            const QString cpsPath = pictura::sessionFilePath();
            QJsonObject cpsRaw;
            {
                QFile cpsFile(cpsPath);
                if (cpsFile.open(QIODevice::ReadOnly)) {
                    cpsRaw = QJsonDocument::fromJson(cpsFile.readAll()).object();
                }
            }
            cpsRaw.insert(QStringLiteral("colorPolicy"), 7);
            bool cpsWrote = false;
            {
                QFile cpsFile(cpsPath);
                if (cpsFile.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
                    cpsWrote = cpsFile.write(QJsonDocument(cpsRaw).toJson()) >= 0;
                    cpsFile.close();
                }
            }
            const bool cpsNormalized = cpsWrote && pictura::loadSession().colorPolicy == 0;
            const bool cpsDefault = pictura::SessionState{}.colorPolicy == 0;

            ST_BEGIN("session_color_policy");
            ST_PASS("session_color_policy convert=%d off=%d normalized=%d default=%d",
                    (cpsSavedConvert && cpsRoundConvert) ? 1 : 0,
                    (cpsSavedOff && cpsRoundOff) ? 1 : 0, cpsNormalized ? 1 : 0,
                    cpsDefault ? 1 : 0);
            if (!cpsSavedConvert || !cpsRoundConvert || !cpsSavedOff || !cpsRoundOff
                || !cpsNormalized || !cpsDefault) {
                return pictura::selfTest().fail(462, "session color policy");
            }
        }

        // lpr_raster_export (532): Save writes the format named by the path (a
        // layered document saved to .png clears modified and re-saves as PNG);
        // Export As and Quick Export write the flattened composite without
        // touching the document's path, dirty flag, or history.
        {
            const QString stem = QDir::tempPath() + QStringLiteral("/kooka-pictura-export");
            const QString psdPath = stem + QStringLiteral(".psd");
            const QString pngPath = stem + QStringLiteral(".png");
            const QString jpgPath = stem + QStringLiteral(".jpg");
            const QString bmpPath = stem + QStringLiteral(".bmp");
            const QString halfPath = stem + QStringLiteral("-half.png");
            QFile::remove(psdPath);
            QFile::remove(pngPath);
            QFile::remove(jpgPath);
            QFile::remove(bmpPath);
            QFile::remove(halfPath);

            const bool created = frame.newDocument(QStringLiteral("export"), 4, 4,
                                                   QStringLiteral("rgb"), 8,
                                                   QStringLiteral("white"));
            pictura::PictureView* view = frame.activeView();
            const int exportIndex = frame.activeDocumentIndex();
            const bool defaultFormat =
                created && view && output_format(*view) == QStringLiteral("psd");
            const bool commandsRegistered = frame.registry()
                && frame.registry()->action(QString::fromLatin1(command_ids::FileExportAs))
                    != nullptr
                && frame.registry()->action(QString::fromLatin1(command_ids::FileQuickExportPng))
                    != nullptr;

            // A flat document plain-Saves to its raster path; a layered one must
            // fall back to the format-aware Save As dialog.
            const bool flatNeedsDialog =
                view && !savePathNeedsFormatDialog(pngPath, view, false);

            // Save As names the format by suffix; a layered document flattens.
            view->add_adjustment(QStringLiteral("invert"));
            const bool layeredNeedsDialog = view
                && savePathNeedsFormatDialog(pngPath, view, false)
                && !savePathNeedsFormatDialog(psdPath, view, true);
            const bool savedPng = defaultFormat && frame.saveActiveAs(pngPath);
            const bool pngClean = savedPng && view && !view->is_dirty();
            QImage pngImage(pngPath);
            const QColor pngPixel = pngImage.pixelColor(0, 0);
            const bool pngWritten = pngClean && !pngImage.isNull() && pngImage.width() == 4
                && pngImage.height() == 4;
            // The white document was inverted before saving, so the written
            // pixels must be black -- a real pixel check, not just dimensions.
            const bool pngPixels = pngWritten && pngPixel.red() < 4 && pngPixel.green() < 4
                && pngPixel.blue() < 4;

            // Reopen the written PNG: it must decode through the import edge and
            // report PNG as its remembered output format.
            const int docsBefore = frame.documentCount();
            const bool reopened = pngWritten && frame.openDocumentAtPath(pngPath);
            pictura::PictureView* imported = frame.activeView();
            const bool importedPng = reopened && frame.documentCount() == docsBefore + 1
                && imported && output_format(*imported) == QStringLiteral("png")
                && imported->document_width() == 4 && imported->document_height() == 4;
            // A bare import (one background layer) keeps its source format as
            // the Save As default, and needs no flatten warning.
            const bool bareDefault = reopened && imported
                && defaultSaveFilterIndex(imported) == 2;
            const bool importedFlatWarn = imported
                && !saveAsNeedsFeatureWarning(pngPath, imported);
            if (reopened) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
                frame.setActiveDocumentIndex(exportIndex);
                view = frame.activeView();
            }

            // Export As: PNG/JPEG/BMP plus a 50% scaled PNG, all read-only.
            const int historyBefore = view ? view->history_count() : -1;
            const QString pathBefore = view ? view->file_path() : QString();
            const bool wasDirty = view && view->is_dirty();
            const bool exportPng =
                view && export_image(*view, pngPath, QStringLiteral("PNG"), -1, 100);
            const bool exportJpg =
                view && export_image(*view, jpgPath, QStringLiteral("JPG"), 90, 100);
            const bool exportBmp =
                view && export_image(*view, bmpPath, QStringLiteral("BMP"), -1, 100);
            const bool exportHalf =
                view && export_image(*view, halfPath, QStringLiteral("PNG"), -1, 50);
            const bool exportFiles = exportPng && exportJpg && exportBmp
                && !QImage(pngPath).isNull() && !QImage(jpgPath).isNull()
                && !QImage(bmpPath).isNull();
            const bool scaled = exportHalf && QImage(halfPath).width() == 2
                && QImage(halfPath).height() == 2;
            const bool unmutated = view && view->history_count() == historyBefore
                && view->file_path() == pathBefore && view->is_dirty() == wasDirty;

            // A layered document defaults Save As to PSD, not the import format.
            const bool layeredDefault = view && defaultSaveFilterIndex(view) == 0;

            // The flatten warning fires for a raster extension on a layered
            // document, not for PSD/PSB or a flat import.
            const bool warnPredicate = view && saveAsNeedsFeatureWarning(pngPath, view)
                && !saveAsNeedsFeatureWarning(psdPath, view) && importedFlatWarn;

            // Revert must reload a raster-recorded document in place (keeping the
            // recorded path in sync) instead of failing silently through the
            // PSD-only reader.
            if (view) {
                view->add_adjustment(QStringLiteral("invert"));
            }
            const bool reverted = frame.revertActive() && view && !view->is_dirty()
                && output_format(*view) == QStringLiteral("png")
                && view->file_path() == pngPath;

            // A .psb path forces a version-2 PSB container ("8BPS" + version 2).
            const QString psbPath = stem + QStringLiteral(".psb");
            QFile::remove(psbPath);
            const bool savedPsb = frame.saveActiveAs(psbPath);
            QByteArray psbHead;
            {
                QFile psb(psbPath);
                if (psb.open(QIODevice::ReadOnly)) {
                    psbHead = psb.read(6);
                }
            }
            const bool psbWritten = savedPsb && psbHead.size() == 6
                && psbHead.left(4) == QByteArrayLiteral("8BPS")
                && static_cast<unsigned char>(psbHead.at(5)) == 2;

            // The file-type combo is non-editable and rolls with a repeated key.
            QComboBox combo;
            combo.addItems(pictura::saveFileFilters());
            combo.setCurrentIndex(2); // PNG
            pictura::installRollingTypeAhead(&combo);
            auto pressP = [&combo] {
                QKeyEvent key(QEvent::KeyPress, Qt::Key_P, Qt::NoModifier, QStringLiteral("p"));
                QCoreApplication::sendEvent(&combo, &key);
            };
            pressP();
            const int roll1 = combo.currentIndex(); // Photoshop
            pressP();
            const int roll2 = combo.currentIndex(); // Photoshop Large Format
            pressP();
            const int roll3 = combo.currentIndex(); // PNG
            const bool rolls = !combo.isEditable() && roll1 == 0 && roll2 == 1 && roll3 == 2;

            // Quick Export writes <source stem>.png beside the document's file.
            const bool savedPsd = frame.saveActiveAs(psdPath);
            QFile::remove(pngPath);
            const bool quickExported = savedPsd && quickExportPngFromView(nullptr, view);
            const bool quickWritten = quickExported && QFile::exists(pngPath)
                && QImage(pngPath).width() == 4;

            // Alpha: PNG keeps it, JPEG and BMP flatten onto opaque white.
            const bool alphaCreated = frame.newDocument(QStringLiteral("alpha"), 2, 1,
                                                        QStringLiteral("rgb"), 8,
                                                        QStringLiteral("transparent"));
            pictura::PictureView* alphaView = frame.activeView();
            const int alphaIndex = frame.activeDocumentIndex();
            const QString alphaPng = stem + QStringLiteral("-alpha.png");
            const QString alphaJpg = stem + QStringLiteral("-alpha.jpg");
            const QString alphaBmp = stem + QStringLiteral("-alpha.bmp");
            const QString badPath = QStringLiteral("/nonexistent-kooka-pictura/x.png");
            QFile::remove(alphaPng);
            QFile::remove(alphaJpg);
            QFile::remove(alphaBmp);
            const bool alphaPngOk = alphaCreated && alphaView
                && export_image(*alphaView, alphaPng, QStringLiteral("PNG"), -1, 100)
                && QImage(alphaPng).pixelColor(0, 0).alpha() == 0;
            const bool alphaJpgOk = alphaCreated && alphaView
                && export_image(*alphaView, alphaJpg, QStringLiteral("JPG"), 90, 100)
                && QImage(alphaJpg).pixelColor(0, 0).alpha() == 255;
            const bool alphaBmpOk = alphaCreated && alphaView
                && export_image(*alphaView, alphaBmp, QStringLiteral("BMP"), -1, 100)
                && QImage(alphaBmp).pixelColor(0, 0).alpha() == 255;
            const bool unwritable = alphaCreated && alphaView
                && !export_image(*alphaView, badPath, QStringLiteral("PNG"), -1, 100)
                && !QFile::exists(badPath);
            if (alphaCreated) {
                frame.closeDocument(alphaIndex, false);
            }

            // Export As refuses without a document.
            auto* emptyView = new pictura::PictureView(&frame);
            const bool emptyExport = !exportAsFromView(nullptr, emptyView);
            delete emptyView;

            // The Places sidebar exposes the filesystem root and the home folder.
            QStringList placePaths;
            for (const QUrl& url : pictura::fileDialogPlaces()) {
                placePaths.append(url.toLocalFile());
            }
            // QDir::rootPath() is "/" on Unix and the drive root ("C:/") on
            // Windows, matching the volume root the sidebar adds there.
            const bool placesOk = placePaths.contains(QDir::rootPath())
                && placePaths.contains(QDir::homePath());

            // The hybrid dialog switch: default follows the sandbox, and the
            // override forces the portal path.
            const QByteArray savedFlatpak = qgetenv("FLATPAK_ID");
            const QByteArray savedSnap = qgetenv("SNAP");
            const QByteArray savedSnapName = qgetenv("SNAP_NAME");
            qunsetenv("FLATPAK_ID");
            qunsetenv("SNAP");
            qunsetenv("SNAP_NAME");
            qunsetenv("PICTURA_PORTAL_FILE_DIALOG");
            const bool portalDefault = !pictura::usesPortalFileDialog();
            qputenv("PICTURA_PORTAL_FILE_DIALOG", "1");
            const bool portalOverride = pictura::usesPortalFileDialog();
            qunsetenv("PICTURA_PORTAL_FILE_DIALOG");
            if (!savedFlatpak.isEmpty()) {
                qputenv("FLATPAK_ID", savedFlatpak);
            }
            if (!savedSnap.isEmpty()) {
                qputenv("SNAP", savedSnap);
            }
            if (!savedSnapName.isEmpty()) {
                qputenv("SNAP_NAME", savedSnapName);
            }
            const bool portalDecision = portalDefault && portalOverride;

            // Places: a recent location's directory appears, a nonexistent place
            // is dropped, and every entry is an existing directory.
            const QString recentDir = QDir::tempPath() + QStringLiteral("/kooka-places");
            QDir().mkpath(recentDir);
            // fileDialogPlaces() canonicalizes each place (symlinks resolved), so
            // match against the canonical temp dir: on macOS QDir::tempPath() is
            // the /var symlink while the place resolves to /private/var.
            const QString recentDirKey = QFileInfo(recentDir).canonicalFilePath();
            const QString recentFile = recentDir + QStringLiteral("/doc.psd");
            {
                QFile seeded(recentFile);
                seeded.open(QIODevice::WriteOnly);
                seeded.write("x");
            }
            pictura::setFileDialogRecentPaths(
                {recentFile, QStringLiteral("/nonexistent-kooka-pictura/x.png")});
            bool hasRecent = false;
            bool hasMissing = false;
            bool allDirs = true;
            for (const QUrl& url : pictura::fileDialogPlaces()) {
                const QString place = url.toLocalFile();
                hasRecent = hasRecent || place == recentDirKey;
                hasMissing = hasMissing
                    || place.startsWith(QStringLiteral("/nonexistent-kooka-pictura"));
                allDirs = allDirs && QFileInfo(place).isDir();
            }
            const bool placesDetails = hasRecent && !hasMissing && allDirs;
            pictura::setFileDialogRecentPaths({});
            QFile::remove(recentFile);
            QDir().rmdir(recentDir);

            // Open As Smart Object accepts a raster source (CS6 parity): a PNG
            // becomes a single editable smart-object layer.
            const bool soOpened = frame.openAsSmartObjectPath(pngPath);
            pictura::PictureView* soView = frame.activeView();
            const bool smartObjectOpen = soOpened && soView && soView->layer_count() == 1
                && soView->layer_can_edit_smart_object_contents(QStringLiteral("0"));
            if (soOpened) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }

            const bool exportOk = defaultFormat && commandsRegistered && pngWritten
                && pngPixels && importedPng && bareDefault && layeredDefault && psbWritten
                && rolls && flatNeedsDialog && layeredNeedsDialog && exportFiles && scaled
                && unmutated && quickWritten && warnPredicate && reverted && alphaPngOk
                && alphaJpgOk && alphaBmpOk && unwritable && emptyExport
                && placesOk && portalDecision && placesDetails && smartObjectOpen;
            ST_BEGIN("raster_export");
            ST_PASS("raster_export default=%d commands=%d png=%d pixels=%d reopened=%d bare=%d "
                    "layered=%d psb=%d rolls=%d savedlg=%d/%d files=%d scaled=%d unmutated=%d "
                    "quick=%d warn=%d revert=%d alpha=%d/%d/%d unwritable=%d empty=%d "
                    "places=%d portal=%d places2=%d smartobj=%d",
                    defaultFormat ? 1 : 0, commandsRegistered ? 1 : 0, pngWritten ? 1 : 0,
                    pngPixels ? 1 : 0, importedPng ? 1 : 0, bareDefault ? 1 : 0,
                    layeredDefault ? 1 : 0, psbWritten ? 1 : 0, rolls ? 1 : 0,
                    flatNeedsDialog ? 1 : 0, layeredNeedsDialog ? 1 : 0, exportFiles ? 1 : 0,
                    scaled ? 1 : 0, unmutated ? 1 : 0, quickWritten ? 1 : 0,
                    warnPredicate ? 1 : 0, reverted ? 1 : 0, alphaPngOk ? 1 : 0,
                    alphaJpgOk ? 1 : 0, alphaBmpOk ? 1 : 0, unwritable ? 1 : 0,
                    emptyExport ? 1 : 0, placesOk ? 1 : 0,
                    portalDecision ? 1 : 0, placesDetails ? 1 : 0, smartObjectOpen ? 1 : 0);
            for (const QString& path : {psdPath, psbPath, pngPath, jpgPath, bmpPath, halfPath,
                                        alphaPng, alphaJpg, alphaBmp}) {
                QFile::remove(path);
            }
            if (view) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!exportOk) {
                return pictura::selfTest().fail(532, "raster export");
            }
        }

        {
            // dialog_last_dir (534): an empty-directory dialog must start in the
            // folder Qt last visited. Qt stores it as a file:// URL but reads it
            // back as a plain filesystem path (resetting to the working
            // directory), so we convert the same key ourselves.
            const QString tmp = QDir::tempPath() + QStringLiteral("/kooka-dialog-dir");
            QDir().mkpath(tmp);
            const QString urlForm = QUrl::fromLocalFile(tmp).toString();
            const bool urlOk = dialogDirectoryFromStored(urlForm) == tmp;
            const bool pathOk = dialogDirectoryFromStored(tmp) == tmp;
            const bool missingOk =
                dialogDirectoryFromStored(tmp + QStringLiteral("-nope")).isEmpty();
            const bool emptyOk = dialogDirectoryFromStored(QString()).isEmpty();
            // The document tier: a document's folder, empty when there is none.
            const bool documentOk = documentDirectory(tmp + QStringLiteral("/c.psd")) == tmp
                && documentDirectory(QString()).isEmpty();

            QSettings settings(QSettings::UserScope, QStringLiteral("QtProject"));
            settings.beginGroup(QStringLiteral("FileDialog"));
            const QVariant saved = settings.value(QStringLiteral("lastVisited"));
            settings.setValue(QStringLiteral("lastVisited"), urlForm);
            settings.sync();
            const bool rememberedOk = rememberedFileDialogDirectory() == tmp;
            // A remembered folder that was moved or deleted falls back to home.
            settings.setValue(QStringLiteral("lastVisited"),
                              urlForm + QStringLiteral("-gone"));
            settings.sync();
            const bool homeOk = defaultFileDialogDirectory() == QDir::homePath();
            if (saved.isValid()) {
                settings.setValue(QStringLiteral("lastVisited"), saved);
            } else {
                settings.remove(QStringLiteral("lastVisited"));
            }
            settings.endGroup();
            settings.sync();
            QDir().rmdir(tmp);

            const bool dirsOk = urlOk && pathOk && missingOk && emptyOk && documentOk
                && rememberedOk && homeOk;
            ST_BEGIN("dialog_last_dir");
            ST_PASS("dialog_last_dir url=%d path=%d missing=%d empty=%d doc=%d stored=%d "
                    "home=%d",
                    urlOk ? 1 : 0, pathOk ? 1 : 0, missingOk ? 1 : 0, emptyOk ? 1 : 0,
                    documentOk ? 1 : 0, rememberedOk ? 1 : 0, homeOk ? 1 : 0);
            if (!dirsOk) {
                return pictura::selfTest().fail(534, "dialog last dir");
            }
        }

        return 0;
}

} // namespace pictura
