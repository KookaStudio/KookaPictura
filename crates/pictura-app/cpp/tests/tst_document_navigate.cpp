// The multi-window Hand and Zoom options (#500): Scroll All Windows and its
// Shift form, Zoom All Windows and its Shift form, Resize Windows To Fit, the
// Zoom Resizes Windows preference, and their persistence. Each test function
// builds its own frame; floats are hosted in the frame, so their geometry is
// the frame's.

#include <QtTest/QtTest>

#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QStackedWidget>

#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "document_float_window.h"
#include "frame.h"
#include "image_view.h"
#include "panels/panel_column.h"
#include "preferences_dialog.h"
#include "session.h"
#include "tool_options.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

const QString kScrollAll = QStringLiteral("optionsHandScrollAll");
const QString kResizeToFit = QStringLiteral("optionsZoomResizeToFit");
const QString kZoomAll = QStringLiteral("optionsZoomAllWindows");
const QString kZoomResizesPref = QStringLiteral("zoomResizesWindows");

QPointF widgetCentre(const ImageView* canvas)
{
    return QRectF(canvas->rect()).center();
}

QRect globalRect(const QWidget* widget)
{
    return QRect(widget->mapToGlobal(QPoint(0, 0)), widget->size());
}

void sendMouse(QWidget* target, QEvent::Type type, const QPoint& local, Qt::MouseButton button,
               Qt::MouseButtons held, Qt::KeyboardModifiers mods)
{
    QMouseEvent event(type, QPointF(local), QPointF(target->mapToGlobal(local)), button, held,
                      mods);
    QApplication::sendEvent(target, &event);
}

// A left-button drag on `canvas` from its centre by `delta`, in four steps,
// with `mods` held throughout: a Hand drag while the Hand tool is active.
void handDrag(ImageView* canvas, const QPoint& delta, Qt::KeyboardModifiers mods)
{
    const QPoint from = canvas->rect().center();
    sendMouse(canvas, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton, mods);
    for (int i = 1; i <= 4; ++i) {
        sendMouse(canvas, QEvent::MouseMove, from + delta * i / 4, Qt::NoButton, Qt::LeftButton,
                  mods);
    }
    sendMouse(canvas, QEvent::MouseButtonRelease, from + delta, Qt::LeftButton, Qt::NoButton,
              mods);
}

// A Zoom tool click at the centre of the active canvas, as the canvas reports it.
void zoomClick(ImageView* canvas, Qt::KeyboardModifiers mods)
{
    const QPointF imagePoint = canvas->widgetToImage(widgetCentre(canvas));
    emit canvas->mousePressed(imagePoint, Qt::LeftButton, int(mods));
    emit canvas->mouseReleased(imagePoint);
}

bool samePoint(const QPointF& a, const QPointF& b)
{
    return qAbs(a.x() - b.x()) <= 0.5 && qAbs(a.y() - b.y()) <= 0.5;
}

} // namespace

class DocumentNavigateTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void scrollAllWindows();
    void shiftScrollsAllWindows();
    void plainDragScrollsOneWindow();
    void zoomAllWindows();
    void shiftClickZoomsAllWindows();
    void resizeWindowsToFit();
    void zoomResizesWindowsPref();
    void handZoomOptionsPersist();

private:
    bool newDoc(const QString& name, int width = 400, int height = 300);
    QStackedWidget* documentArea() const;
    QCheckBox* option(const QString& name) const;
    PreferencesDialog* generalPreferences();
    // Float documents 0 and 1 side by side across the document area, both
    // zoomed to 400% and centred, with document 0 active.
    bool twoFloatsSideBySide();
    // Float document `index` small in the document area's top-left corner,
    // its canvas at 50%, and make it active.
    bool smallFloat(int index);
    void closeAll();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

bool DocumentNavigateTest::newDoc(const QString& name, int width, int height)
{
    if (!window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    window_->setDocumentNameForTest(window_->documentCount() - 1, name);
    return true;
}

QStackedWidget* DocumentNavigateTest::documentArea() const
{
    return window_->findChild<QStackedWidget*>(QStringLiteral("documentArea"));
}

QCheckBox* DocumentNavigateTest::option(const QString& name) const
{
    return window_->findChild<QCheckBox*>(name);
}

PreferencesDialog* DocumentNavigateTest::generalPreferences()
{
    window_->registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesGeneral));
    QCoreApplication::processEvents();
    return window_->preferencesDialog();
}

bool DocumentNavigateTest::twoFloatsSideBySide()
{
    if (!newDoc(QStringLiteral("Left")) || !newDoc(QStringLiteral("Right"))
        || !window_->floatDocumentForTest(0) || !window_->floatDocumentForTest(1)) {
        return false;
    }
    DocumentFloatWindow* left = window_->floatWindowForTest(0);
    DocumentFloatWindow* right = window_->floatWindowForTest(1);
    const QRect area = globalRect(documentArea());
    if (!left || !right || area.isEmpty()) {
        return false;
    }
    const QSize half(area.width() / 2, area.height());
    left->setGlobalGeometry(QRect(area.topLeft(), half));
    right->setGlobalGeometry(QRect(area.topLeft() + QPoint(half.width(), 0), half));
    window_->setActiveDocumentIndex(0);
    QCoreApplication::processEvents();
    for (int i = 0; i < 2; ++i) {
        ImageView* canvas = window_->canvasAt(i);
        if (!canvas || !canvas->isVisible()) {
            return false;
        }
        canvas->setZoom(4.0, widgetCentre(canvas));
        canvas->centerOnImageRect(QRectF(199.5, 149.5, 1.0, 1.0));
    }
    return window_->imageView() == window_->canvasAt(0);
}

bool DocumentNavigateTest::smallFloat(int index)
{
    if (!window_->floatDocumentForTest(index)) {
        return false;
    }
    DocumentFloatWindow* floating = window_->floatWindowForTest(index);
    const QRect area = globalRect(documentArea());
    if (!floating || area.isEmpty()) {
        return false;
    }
    floating->setGlobalGeometry(QRect(area.topLeft() + QPoint(20, 20), QSize(220, 180)));
    window_->setActiveDocumentIndex(index);
    QCoreApplication::processEvents();
    ImageView* canvas = window_->canvasAt(index);
    if (!canvas || window_->imageView() != canvas) {
        return false;
    }
    canvas->setZoom(0.5, widgetCentre(canvas));
    return true;
}

void DocumentNavigateTest::closeAll()
{
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
}

void DocumentNavigateTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    setUnsavedPromptInteractive(false);
}

void DocumentNavigateTest::init()
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

void DocumentNavigateTest::cleanup()
{
    if (window_) {
        // Turn the options back off and save, so the shared session store the
        // next frame loads keeps the defaults.
        for (const QString& name : {kScrollAll, kResizeToFit, kZoomAll}) {
            if (QCheckBox* box = option(name)) {
                box->setChecked(false);
            }
        }
        if (PreferencesDialog* prefs = window_->preferencesDialog()) {
            prefs->setCheckboxForTest(kZoomResizesPref, false);
            prefs->close();
        }
        window_->setActiveTool(ToolId::Move);
        window_->saveSession();
        window_->cancelAllOpens();
        closeAll();
    }
    window_.reset();
    PanelFloat::setForceChildOverlayForTest(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentNavigateTest::scrollAllWindows()
{
    QVERIFY(twoFloatsSideBySide());
    QCheckBox* scrollAll = option(kScrollAll);
    QVERIFY(scrollAll != nullptr);
    scrollAll->setChecked(true);
    window_->setActiveTool(ToolId::Hand);
    ImageView* dragged = window_->canvasAt(0);
    ImageView* other = window_->canvasAt(1);
    QVERIFY(dragged->panEnabled());
    const QPointF draggedBefore = dragged->offset();
    const QPointF otherBefore = other->offset();
    const int history = window_->viewAt(0)->history_index();

    handDrag(dragged, QPoint(40, 0), Qt::NoModifier);
    QVERIFY2(samePoint(dragged->offset(), draggedBefore + QPointF(40, 0)),
             "the dragged window moves 40 px right");
    QVERIFY2(samePoint(other->offset(), otherBefore + QPointF(40, 0)),
             "the other window moves 40 px right with it");
    QCOMPARE(window_->viewAt(0)->history_index(), history);
}

void DocumentNavigateTest::shiftScrollsAllWindows()
{
    QVERIFY(twoFloatsSideBySide());
    QCheckBox* scrollAll = option(kScrollAll);
    QVERIFY(scrollAll != nullptr);
    QVERIFY(!scrollAll->isChecked());
    window_->setActiveTool(ToolId::Hand);
    ImageView* dragged = window_->canvasAt(0);
    ImageView* other = window_->canvasAt(1);
    const QPointF draggedBefore = dragged->offset();
    const QPointF otherBefore = other->offset();

    handDrag(dragged, QPoint(-24, 32), Qt::ShiftModifier);
    QVERIFY(samePoint(dragged->offset(), draggedBefore + QPointF(-24, 32)));
    QVERIFY2(samePoint(other->offset(), otherBefore + QPointF(-24, 32)),
             "Shift scrolls the other window with the option off");
}

void DocumentNavigateTest::plainDragScrollsOneWindow()
{
    QVERIFY(twoFloatsSideBySide());
    QCheckBox* scrollAll = option(kScrollAll);
    QVERIFY(scrollAll != nullptr);
    QVERIFY(!scrollAll->isChecked());
    window_->setActiveTool(ToolId::Hand);
    ImageView* dragged = window_->canvasAt(0);
    ImageView* other = window_->canvasAt(1);
    const QPointF draggedBefore = dragged->offset();
    const QPointF otherBefore = other->offset();

    handDrag(dragged, QPoint(40, 0), Qt::NoModifier);
    QVERIFY(samePoint(dragged->offset(), draggedBefore + QPointF(40, 0)));
    QVERIFY2(samePoint(other->offset(), otherBefore), "only the dragged window moves");
}

void DocumentNavigateTest::zoomAllWindows()
{
    QVERIFY(newDoc(QStringLiteral("Other")));
    QVERIFY(newDoc(QStringLiteral("Clicked")));
    QCoreApplication::processEvents();
    ImageView* other = window_->canvasAt(0);
    ImageView* clicked = window_->canvasAt(1);
    QVERIFY(other && clicked);
    QCOMPARE(window_->imageView(), clicked);
    clicked->setZoom(1.0, widgetCentre(clicked));
    other->setZoom(0.5, widgetCentre(other));
    QCheckBox* zoomAll = option(kZoomAll);
    QVERIFY(zoomAll != nullptr);
    zoomAll->setChecked(true);
    window_->setActiveTool(ToolId::Zoom);
    const int history = window_->viewAt(1)->history_index();

    zoomClick(clicked, Qt::NoModifier);
    const double factor = clicked->zoom() / 1.0;
    QVERIFY2(factor > 1.0, "the click zooms the clicked window in");
    QVERIFY2(qAbs(other->zoom() / 0.5 - factor) < 1e-6,
             "the other window zooms by the same factor");
    QCOMPARE(window_->viewAt(1)->history_index(), history);
}

void DocumentNavigateTest::shiftClickZoomsAllWindows()
{
    QVERIFY(newDoc(QStringLiteral("Other")));
    QVERIFY(newDoc(QStringLiteral("Clicked")));
    QCoreApplication::processEvents();
    ImageView* other = window_->canvasAt(0);
    ImageView* clicked = window_->canvasAt(1);
    QVERIFY(other && clicked);
    clicked->setZoom(2.0, widgetCentre(clicked));
    other->setZoom(0.25, widgetCentre(other));
    QCheckBox* zoomAll = option(kZoomAll);
    QVERIFY(zoomAll != nullptr);
    QVERIFY(!zoomAll->isChecked());
    window_->setActiveTool(ToolId::Zoom);

    zoomClick(clicked, Qt::ShiftModifier);
    const double factor = clicked->zoom() / 2.0;
    QVERIFY(factor > 1.0);
    QVERIFY2(qAbs(other->zoom() / 0.25 - factor) < 1e-6,
             "Shift+click zooms the other window by the same factor");

    // Without Shift and with the option off, only the clicked window zooms.
    const double otherZoom = other->zoom();
    zoomClick(clicked, Qt::NoModifier);
    QCOMPARE(other->zoom(), otherZoom);
}

void DocumentNavigateTest::resizeWindowsToFit()
{
    QVERIFY(newDoc(QStringLiteral("Floating")));
    QVERIFY(newDoc(QStringLiteral("Tabbed")));
    QVERIFY(smallFloat(0));
    QCheckBox* resize = option(kResizeToFit);
    QVERIFY(resize != nullptr);
    resize->setChecked(true);
    window_->setActiveTool(ToolId::Zoom);
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    const QRect area = globalRect(documentArea());
    const QRect before = floating->globalGeometry();

    zoomClick(window_->canvasAt(0), Qt::NoModifier);
    QCoreApplication::processEvents();
    const QRect after = floating->globalGeometry();
    QVERIFY2(after.width() > before.width() && after.height() > before.height(),
             "the float grows to fit the zoomed image");
    QVERIFY2(area.contains(after), "the float stays inside the document area");

    // A tabbed window zoomed the same way keeps its size.
    window_->setActiveDocumentIndex(1);
    QCoreApplication::processEvents();
    ImageView* tabbed = window_->canvasAt(1);
    QCOMPARE(window_->imageView(), tabbed);
    tabbed->setZoom(0.5, widgetCentre(tabbed));
    const QSize tabSize = tabbed->size();
    const QRect floatBefore = floating->globalGeometry();
    zoomClick(tabbed, Qt::NoModifier);
    QCoreApplication::processEvents();
    QVERIFY(tabbed->zoom() > 0.5);
    QCOMPARE(tabbed->size(), tabSize);
    QCOMPARE(floating->globalGeometry(), floatBefore);
}

void DocumentNavigateTest::zoomResizesWindowsPref()
{
    QVERIFY(newDoc(QStringLiteral("Floating")));
    PreferencesDialog* prefs = generalPreferences();
    QVERIFY(prefs != nullptr);
    QVERIFY(!prefs->checkboxForTest(kZoomResizesPref));
    QVERIFY(prefs->setCheckboxForTest(kZoomResizesPref, true));
    QVERIFY(window_->zoomResizesWindows());
    prefs->close();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
    QVERIFY(smallFloat(0));
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    ImageView* canvas = window_->canvasAt(0);
    const QRect area = globalRect(documentArea());
    const QRect before = floating->globalGeometry();

    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_Equal, Qt::ControlModifier);
    QCoreApplication::processEvents();
    QVERIFY(canvas->zoom() > 0.5);
    const QRect resized = floating->globalGeometry();
    QVERIFY2(resized.size() != before.size(), "Ctrl+= resizes the float with the pref on");
    QVERIFY(area.contains(resized));

    QVERIFY(prefs->setCheckboxForTest(kZoomResizesPref, false));
    QVERIFY(!window_->zoomResizesWindows());
    const double zoom = canvas->zoom();
    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_Equal, Qt::ControlModifier);
    QCoreApplication::processEvents();
    QVERIFY(canvas->zoom() > zoom);
    QCOMPARE(floating->globalGeometry(), resized);
}

void DocumentNavigateTest::handZoomOptionsPersist()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    {
        auto first = pictura::test::makeMainWindow();
        auto* tools = first->findChild<ToolController*>();
        QVERIFY(tools != nullptr);
        HandGroup* hand = tools->optionsFor<HandGroup>(ToolId::Hand);
        ZoomGroup* zoom = tools->optionsFor<ZoomGroup>(ToolId::Zoom);
        QVERIFY(hand && zoom);
        QVERIFY2(!hand->get().scrollAll && !zoom->get().resizeToFit && !zoom->get().zoomAll,
                 "all three options start off");
        QVERIFY2(!first->zoomResizesWindows(), "Zoom Resizes Windows starts off");
        for (const QString& name : {kScrollAll, kResizeToFit, kZoomAll}) {
            auto* box = first->findChild<QCheckBox*>(name);
            QVERIFY2(box != nullptr, qPrintable(name));
            box->setChecked(true);
        }
        QVERIFY(hand->get().scrollAll && zoom->get().resizeToFit && zoom->get().zoomAll);
        first->registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesGeneral));
        PreferencesDialog* prefs = first->preferencesDialog();
        QVERIFY(prefs != nullptr);
        QVERIFY(prefs->setCheckboxForTest(kZoomResizesPref, true));
        first->saveSession();
        prefs->close();
    }

    const SessionState state = loadSession();
    QVERIFY(state.zoomResizesWindows);
    auto reloaded = pictura::test::makeMainWindow();
    QVERIFY(reloaded->zoomResizesWindows());
    auto* tools = reloaded->findChild<ToolController*>();
    QVERIFY(tools != nullptr);
    QVERIFY(tools->optionsFor<HandGroup>(ToolId::Hand)->get().scrollAll);
    QVERIFY(tools->optionsFor<ZoomGroup>(ToolId::Zoom)->get().resizeToFit);
    QVERIFY(tools->optionsFor<ZoomGroup>(ToolId::Zoom)->get().zoomAll);
    for (const QString& name : {kScrollAll, kResizeToFit, kZoomAll}) {
        auto* box = reloaded->findChild<QCheckBox*>(name);
        QVERIFY2(box && box->isChecked(), qPrintable(name));
    }
}

QTEST_MAIN(DocumentNavigateTest)
#include "tst_document_navigate.moc"
