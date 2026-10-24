// Window > Arrange (#500) and the document window preferences (#317): Float in
// Window, Float All in Windows, Consolidate All to Tabs, Cascade, the Match
// commands, Open Documents As Tabs, Enable Floating Document Window Docking,
// and crash recovery reopening as tabs. Each test function builds its own
// frame; floats are hosted in the frame, so their geometry is the frame's.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QLineF>
#include <QtCore/QTemporaryDir>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>

#include "api/document_api.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "document_float_window.h"
#include "frame.h"
#include "image_view.h"
#include "panels/panel_column.h"
#include "preferences_dialog.h"
#include "recovery_store.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_core/recovery.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

const QString kFloatInWindow = QStringLiteral("window.arrange.float.in.window");
const QString kFloatAll = QStringLiteral("window.arrange.float.all.in.windows");
const QString kConsolidate = QStringLiteral("window.arrange.consolidate.all.to.tabs");
const QString kCascade = QStringLiteral("window.arrange.cascade");
const QString kMatchZoom = QStringLiteral("window.arrange.match.zoom");
const QString kMatchLocation = QStringLiteral("window.arrange.match.location");
const QString kMatchAll = QStringLiteral("window.arrange.match.all");

const QStringList kArrangeIds = {
    QStringLiteral("window.arrange.cascade"),
    QStringLiteral("window.arrange.tile"),
    QStringLiteral("window.arrange.consolidate.all.to.tabs"),
    QStringLiteral("window.arrange.float.in.window"),
    QStringLiteral("window.arrange.float.all.in.windows"),
    QStringLiteral("window.arrange.match.zoom"),
    QStringLiteral("window.arrange.match.location"),
    QStringLiteral("window.arrange.match.all"),
    QStringLiteral("window.arrange.new.window.for.file"),
};

const QString kTabsPref = QStringLiteral("openDocumentsAsTabs");
const QString kDockingPref = QStringLiteral("enableFloatingDocumentDocking");

QAction* invertAction(const PicturaMainWindow& window)
{
    return window.registry()->action(commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Invert")}));
}

bool writePsd(const QString& path)
{
    PictureView view;
    return document_api::newDocument(view, 4, 4, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white"))
           && document_api::save(view, path);
}

void sendMouse(QWidget* target, QEvent::Type type, const QPoint& global, Qt::MouseButton button,
               Qt::MouseButtons held)
{
    QMouseEvent event(type, QPointF(target->mapFromGlobal(global)), QPointF(global), button,
                      held, Qt::NoModifier);
    QApplication::sendEvent(target, &event);
}

// Press on `target` at `from`, move to `to` in ten steps, and release there.
void dragTo(QWidget* target, const QPoint& from, const QPoint& to)
{
    sendMouse(target, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
    for (int i = 1; i <= 10; ++i) {
        sendMouse(target, QEvent::MouseMove, from + (to - from) * i / 10, Qt::NoButton,
                  Qt::LeftButton);
    }
    sendMouse(target, QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
}

QPointF widgetCentre(const ImageView* canvas)
{
    return QRectF(canvas->rect()).center();
}

// Centre `canvas` on the document point at `fraction` of its width and height.
void centreOnFraction(ImageView* canvas, const QPointF& fraction)
{
    const QSize size = canvas->documentSize();
    const QPointF point(fraction.x() * size.width(), fraction.y() * size.height());
    canvas->centerOnImageRect(QRectF(point - QPointF(0.5, 0.5), QSizeF(1.0, 1.0)));
}

// How far, in widget pixels, `canvas`'s centre is from the document point at
// `fraction` of its width and height.
double centreError(const ImageView* canvas, const QPointF& fraction)
{
    const QSize size = canvas->documentSize();
    const QPointF point(fraction.x() * size.width(), fraction.y() * size.height());
    return QLineF(canvas->imageToWidget(point), widgetCentre(canvas)).length();
}

QRect globalRect(const QWidget* widget)
{
    return QRect(widget->mapToGlobal(QPoint(0, 0)), widget->size());
}

// Move `from`'s snapshots and manifests into a new lock-less session directory,
// as a crashed instance leaves them.
void fakeCrashedSession(const QString& from)
{
    const QString dir = RecoveryStore::rootPath() + QStringLiteral("/999999-1");
    QDir().mkpath(dir);
    for (const QString& name :
         QDir(from).entryList({QStringLiteral("*.psd"), QStringLiteral("*.json")}, QDir::Files)) {
        QFile::copy(from + QLatin1Char('/') + name, dir + QLatin1Char('/') + name);
    }
}

} // namespace

class DocumentArrangeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void arrangeFloatInWindow();
    void arrangeFloatAllThenConsolidate();
    void arrangeDisabledWithoutDocuments();
    void cascadeStepsWindows();
    void cascadeKeepsHeadersReachable();
    void matchZoom();
    void matchLocation();
    void matchAll();

    void prefOpensFloating();
    void prefFloatsOpenedFileAfterLoad();
    void prefDisablesDocking();
    void prefsPersist();
    void prefsDefaultOn();
    void recoveryReopensAsTabs();

private:
    bool newDoc(const QString& name, int width = 32, int height = 32);
    // Refresh the registry, then trigger `id` when it is enabled.
    bool trigger(const QString& id);
    bool enabled(const QString& id);
    QTabBar* tabBar() const;
    QStackedWidget* documentArea() const;
    PreferencesDialog* interfacePreferences();
    void closeAll();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

bool DocumentArrangeTest::newDoc(const QString& name, int width, int height)
{
    return window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                                QStringLiteral("white"));
}

bool DocumentArrangeTest::enabled(const QString& id)
{
    window_->registry()->refresh();
    QAction* action = window_->registry()->action(id);
    return action && action->isEnabled();
}

bool DocumentArrangeTest::trigger(const QString& id)
{
    if (!enabled(id)) {
        return false;
    }
    window_->registry()->action(id)->trigger();
    QCoreApplication::processEvents();
    return true;
}

QTabBar* DocumentArrangeTest::tabBar() const
{
    return window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

QStackedWidget* DocumentArrangeTest::documentArea() const
{
    return window_->findChild<QStackedWidget*>(QStringLiteral("documentArea"));
}

PreferencesDialog* DocumentArrangeTest::interfacePreferences()
{
    window_->registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesInterface));
    QCoreApplication::processEvents();
    return window_->preferencesDialog();
}

void DocumentArrangeTest::closeAll()
{
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
}

void DocumentArrangeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    setUnsavedPromptInteractive(false);
}

void DocumentArrangeTest::init()
{
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    PanelFloat::setForceChildOverlayForTest(true);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void DocumentArrangeTest::cleanup()
{
    if (window_) {
        // Turn both preferences back on, so the shared session store the next
        // frame loads keeps the defaults.
        if (PreferencesDialog* prefs = window_->preferencesDialog()) {
            prefs->setCheckboxForTest(kTabsPref, true);
            prefs->setCheckboxForTest(kDockingPref, true);
            prefs->close();
        }
        window_->cancelAllOpens();
        closeAll();
    }
    window_.reset();
    PanelFloat::setForceChildOverlayForTest(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentArrangeTest::arrangeFloatInWindow()
{
    QVERIFY(newDoc(QStringLiteral("Stays")));
    QVERIFY(newDoc(QStringLiteral("Floats")));
    QCOMPARE(window_->activeDocumentIndex(), 1);

    QVERIFY(trigger(kFloatInWindow));
    QVERIFY(window_->floatWindowForTest(1) != nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(1), -1);
    QVERIFY(window_->floatWindowForTest(0) == nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QVERIFY2(!enabled(kFloatInWindow), "the active window already floats");
}

void DocumentArrangeTest::arrangeFloatAllThenConsolidate()
{
    QVERIFY(newDoc(QStringLiteral("One")));
    QVERIFY(newDoc(QStringLiteral("Two")));
    QVERIFY(newDoc(QStringLiteral("Three")));
    window_->setActiveDocumentIndex(1);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    QVERIFY(trigger(kFloatAll));
    for (int i = 0; i < 3; ++i) {
        QVERIFY(window_->floatWindowForTest(i) != nullptr);
        QCOMPARE(window_->tabOfDocumentForTest(i), -1);
    }
    QCOMPARE(tabBar()->count(), 0);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    QVERIFY(trigger(kConsolidate));
    for (int i = 0; i < 3; ++i) {
        QVERIFY(window_->floatWindowForTest(i) == nullptr);
        QVERIFY(window_->tabOfDocumentForTest(i) >= 0);
    }
    QCOMPARE(tabBar()->count(), 3);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentArrangeTest::arrangeDisabledWithoutDocuments()
{
    QCOMPARE(window_->documentCount(), 0);
    for (const QString& id : kArrangeIds) {
        QVERIFY2(window_->registry()->action(id) != nullptr, qPrintable(id));
        QVERIFY2(!enabled(id), qPrintable(id + QStringLiteral(" enabled without documents")));
    }

    // One window: the float and cascade commands come alive; Match needs two.
    QVERIFY(newDoc(QStringLiteral("Single")));
    for (const QString& id : {kFloatInWindow, kFloatAll, kConsolidate, kCascade}) {
        QVERIFY2(enabled(id), qPrintable(id));
    }
    for (const QString& id : {kMatchZoom, kMatchLocation, kMatchAll}) {
        QVERIFY2(!enabled(id), qPrintable(id + QStringLiteral(" enabled with one window")));
    }
}

void DocumentArrangeTest::cascadeStepsWindows()
{
    QVERIFY(newDoc(QStringLiteral("A")));
    QVERIFY(newDoc(QStringLiteral("B")));
    QVERIFY(newDoc(QStringLiteral("C")));

    QVERIFY(trigger(kCascade));
    QList<QRect> geometries;
    for (int i = 0; i < 3; ++i) {
        DocumentFloatWindow* floating = window_->floatWindowForTest(i);
        QVERIFY(floating != nullptr);
        geometries << floating->globalGeometry();
    }
    QCOMPARE(geometries.at(1).size(), geometries.at(0).size());
    QCOMPARE(geometries.at(2).size(), geometries.at(0).size());
    const QPoint step = geometries.at(1).topLeft() - geometries.at(0).topLeft();
    QVERIFY(step.x() > 0);
    QCOMPARE(step.y(), step.x());
    QCOMPARE(geometries.at(2).topLeft() - geometries.at(1).topLeft(), step);
    QVERIFY(documentArea() != nullptr);
    QVERIFY(globalRect(documentArea()).contains(geometries.at(0).topLeft()));
}

void DocumentArrangeTest::cascadeKeepsHeadersReachable()
{
    window_->resize(760, 520);
    QCoreApplication::processEvents();
    for (int i = 0; i < 12; ++i) {
        QVERIFY(newDoc(QStringLiteral("Doc%1").arg(i + 1)));
    }

    QVERIFY(trigger(kCascade));
    QVERIFY(documentArea() != nullptr);
    const QRect area = globalRect(documentArea());
    for (int i = 0; i < 12; ++i) {
        DocumentFloatWindow* floating = window_->floatWindowForTest(i);
        QVERIFY(floating != nullptr);
        const QRect header = globalRect(floating->header());
        QVERIFY2(area.contains(header),
                 qPrintable(QStringLiteral("header %1 leaves the document area").arg(i)));
    }
}

void DocumentArrangeTest::matchZoom()
{
    QVERIFY(newDoc(QStringLiteral("Other"), 400, 300));
    QVERIFY(newDoc(QStringLiteral("Active"), 400, 300));
    QCoreApplication::processEvents();
    ImageView* other = window_->canvasAt(0);
    ImageView* active = window_->canvasAt(1);
    QVERIFY(other && active);
    other->setZoom(0.5, widgetCentre(other));
    active->setZoom(3.0, widgetCentre(active));
    QCOMPARE(window_->imageView(), active);

    QVERIFY(trigger(kMatchZoom));
    QCOMPARE(active->zoom(), 3.0);
    QCOMPARE(other->zoom(), 3.0);
}

void DocumentArrangeTest::matchLocation()
{
    QVERIFY(newDoc(QStringLiteral("Other"), 400, 300));
    QVERIFY(newDoc(QStringLiteral("Active"), 400, 300));
    QVERIFY(window_->floatDocumentForTest(0));
    window_->setActiveDocumentIndex(1);
    QCoreApplication::processEvents();
    ImageView* other = window_->canvasAt(0);
    ImageView* active = window_->canvasAt(1);
    QVERIFY(other && active);
    other->setZoom(4.0, widgetCentre(other));
    active->setZoom(4.0, widgetCentre(active));
    const QPointF fraction(0.25, 0.75);
    centreOnFraction(active, fraction);
    QVERIFY(centreError(active, fraction) <= 1.0);
    QVERIFY(centreError(other, fraction) > 1.0);

    QVERIFY(trigger(kMatchLocation));
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QVERIFY2(centreError(other, fraction) <= 1.0, "the other centre matches");
    QVERIFY2(centreError(active, fraction) <= 1.0, "the active centre stays");
    QCOMPARE(other->zoom(), 4.0);
}

void DocumentArrangeTest::matchAll()
{
    QVERIFY(newDoc(QStringLiteral("Other"), 400, 300));
    QVERIFY(newDoc(QStringLiteral("Active"), 400, 300));
    QVERIFY(window_->floatDocumentForTest(0));
    window_->setActiveDocumentIndex(1);
    QCoreApplication::processEvents();
    ImageView* other = window_->canvasAt(0);
    ImageView* active = window_->canvasAt(1);
    QVERIFY(other && active);
    other->setZoom(2.0, widgetCentre(other));
    active->setZoom(5.0, widgetCentre(active));
    const QPointF fraction(0.7, 0.3);
    centreOnFraction(active, fraction);
    QVERIFY(centreError(active, fraction) <= 1.0);

    QVERIFY(trigger(kMatchAll));
    QCOMPARE(other->zoom(), 5.0);
    QVERIFY2(centreError(other, fraction) <= 1.0, "the other centre matches");
}

void DocumentArrangeTest::prefOpensFloating()
{
    PreferencesDialog* prefs = interfacePreferences();
    QVERIFY(prefs != nullptr);
    QVERIFY(prefs->checkboxForTest(kTabsPref));
    QVERIFY(prefs->setCheckboxForTest(kTabsPref, false));
    QVERIFY(!window_->openDocumentsAsTabs());

    QVERIFY(newDoc(QStringLiteral("Floating")));
    QVERIFY(window_->floatWindowForTest(0) != nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(0), -1);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(tabBar()->count(), 0);
}

void DocumentArrangeTest::prefFloatsOpenedFileAfterLoad()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("held.psd"));
    QVERIFY(writePsd(path));
    PreferencesDialog* prefs = interfacePreferences();
    QVERIFY(prefs != nullptr);
    QVERIFY(prefs->setCheckboxForTest(kTabsPref, false));

    window_->setOpenDelayForTest(300);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QVERIFY(window_->floatWindowForTest(0) == nullptr);

    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QTRY_VERIFY(window_->floatWindowForTest(0) != nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(0), -1);
    QVERIFY(window_->viewAt(0)->has_document());
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->imageView(), window_->canvasAt(0));
}

void DocumentArrangeTest::prefDisablesDocking()
{
    PreferencesDialog* prefs = interfacePreferences();
    QVERIFY(prefs != nullptr);
    QVERIFY(prefs->checkboxForTest(kDockingPref));
    QVERIFY(prefs->setCheckboxForTest(kDockingPref, false));
    QVERIFY(!window_->floatingDocumentDocking());
    QVERIFY(newDoc(QStringLiteral("Tabbed")));
    QVERIFY(newDoc(QStringLiteral("Floating")));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QWidget* header = floating->header();
    const QPoint from = header->mapToGlobal(QPoint(header->width() / 2, header->height() / 2));
    const QPoint to = bar->mapToGlobal(bar->tabRect(0).center());
    dragTo(header, from, to);
    QCoreApplication::processEvents();
    QCOMPARE(window_->floatWindowForTest(1), floating);
    QCOMPARE(window_->tabOfDocumentForTest(1), -1);
    QCOMPARE(bar->count(), 1);

    QVERIFY(trigger(kConsolidate));
    QVERIFY(window_->floatWindowForTest(1) == nullptr);
    QVERIFY(window_->tabOfDocumentForTest(1) >= 0);
    QCOMPARE(bar->count(), 2);
}

void DocumentArrangeTest::prefsPersist()
{
    PreferencesDialog* prefs = interfacePreferences();
    QVERIFY(prefs != nullptr);
    QVERIFY(prefs->setCheckboxForTest(kTabsPref, false));
    QVERIFY(prefs->setCheckboxForTest(kDockingPref, false));
    window_->saveSession();

    const SessionState state = loadSession();
    QVERIFY(!state.openDocumentsAsTabs);
    QVERIFY(!state.enableFloatingDocumentDocking);
    auto reloaded = pictura::test::makeMainWindow();
    QVERIFY(!reloaded->openDocumentsAsTabs());
    QVERIFY(!reloaded->floatingDocumentDocking());
}

void DocumentArrangeTest::prefsDefaultOn()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    const QString path = sessionFilePath();
    QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
    file.write(R"({"schemaVersion":10,"toolsColumns":2,"useShiftKeyForToolSwitch":false})");
    file.close();

    const SessionState state = loadSession();
    QVERIFY(state.openDocumentsAsTabs);
    QVERIFY(state.enableFloatingDocumentDocking);
    QCOMPARE(state.toolsColumns, 2);
    QVERIFY(!state.useShiftKeyForToolSwitch);
}

void DocumentArrangeTest::recoveryReopensAsTabs()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    {
        auto before = pictura::test::makeMainWindow();
        before->enableRecovery();
        before->registry()->dispatch(
            QString::fromLatin1(command_ids::EditPreferencesInterface));
        PreferencesDialog* prefs = before->preferencesDialog();
        QVERIFY(prefs != nullptr);
        QVERIFY(prefs->setCheckboxForTest(kTabsPref, false));
        QVERIFY(before->newDocument(QString(), 64, 48, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        QVERIFY(before->floatWindowForTest(0) != nullptr);
        before->registry()->refresh();
        QAction* invert = invertAction(*before);
        QVERIFY(invert && invert->isEnabled());
        invert->trigger();
        QVERIFY(before->isDocumentDirty(0));
        before->autosaveRecovery();
        recovery_wait(*before->viewAt(0));
        fakeCrashedSession(before->recoverySessionDirForTest());
        prefs->close();
    }

    auto after = pictura::test::makeMainWindow();
    QVERIFY(!after->openDocumentsAsTabs());
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(after->documentCount(), 1);
    QVERIFY(after->floatWindowForTest(0) == nullptr);
    QCOMPARE(after->tabOfDocumentForTest(0), 0);
    after->shutdownRecovery();
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<DocumentArrangeTest>(argc, argv);
}

#include "tst_document_arrange.moc"
