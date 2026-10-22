// Rulers and guides (#294): View > Rulers, guides dragged out of a ruler and
// moved or deleted with the Move tool, the View guide commands, guides saved in
// a PSD, and the Guides, Grid, & Slices preferences.

#include <QtTest/QtTest>

#include "canvas_ruler.h"
#include "canvas_scrollbars.h"
#include "commands.h"
#include "new_guide_dialog.h"
#include "preferences_dialog.h"
#include "selftest_paint_fixture.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object/guides.cxxqt.h"

#include <QtCore/QTemporaryDir>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QRadioButton>

#include <cmath>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seedImage()
{
    QImage seed(80, 60, QImage::Format_RGB32);
    seed.fill(Qt::gray);
    return seed;
}

QAction* action(pictura::PicturaMainWindow& frame, const char* id)
{
    frame.registry()->refresh();
    return frame.registry()->action(QString::fromLatin1(id));
}

pictura::CanvasScrollBars* hostOf(const Fixture& f)
{
    for (auto* host : f.frame.findChildren<pictura::CanvasScrollBars*>()) {
        if (host->view() == f.canvas) {
            return host;
        }
    }
    return nullptr;
}

void sendMouse(QWidget* target, QEvent::Type type, const QPoint& global, Qt::MouseButtons held)
{
    const QPointF local = target->mapFromGlobal(QPointF(global));
    QMouseEvent event(type, local, QPointF(global), Qt::LeftButton, held, Qt::NoModifier);
    QApplication::sendEvent(target, &event);
}

// Drag from `ruler` (pressed at its local `from`) to the global point `to`.
void dragFromRuler(QWidget* ruler, const QPoint& from, const QPoint& to)
{
    const QPoint start = ruler->mapToGlobal(from);
    sendMouse(ruler, QEvent::MouseButtonPress, start, Qt::LeftButton);
    sendMouse(ruler, QEvent::MouseMove, to, Qt::LeftButton);
    sendMouse(ruler, QEvent::MouseButtonRelease, to, Qt::NoButton);
}

// A real press-move-release on the canvas at image points, so ImageView's own
// pan handling sees it (Fixture::drag emits the tool signals directly).
void dragOnCanvas(pictura::ImageView* canvas, const QPointF& from, const QPointF& to,
                  Qt::KeyboardModifiers mods)
{
    const QPointF a = canvas->imageToWidget(from);
    const QPointF b = canvas->imageToWidget(to);
    QMouseEvent press(QEvent::MouseButtonPress, a, canvas->mapToGlobal(a), Qt::LeftButton,
                      Qt::LeftButton, mods);
    QApplication::sendEvent(canvas, &press);
    QMouseEvent move(QEvent::MouseMove, b, canvas->mapToGlobal(b), Qt::NoButton, Qt::LeftButton,
                     mods);
    QApplication::sendEvent(canvas, &move);
    QMouseEvent release(QEvent::MouseButtonRelease, b, canvas->mapToGlobal(b), Qt::LeftButton,
                        Qt::NoButton, mods);
    QApplication::sendEvent(canvas, &release);
}

QList<double> guide(pictura::PictureView* view, int index)
{
    const ::rust::Vec<double> g = pictura::guide_at(*view, index);
    return QList<double>(g.begin(), g.end());
}

} // namespace

class RulersGuidesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void rulerScale();
    void rulersToggle();
    void togglesDuringLoadingDoNotCrash();
    void dragGuideFromRuler();
    void moveAndDeleteGuide();
    void lockedGuides();
    void clearAndNewGuide();
    void guidesSaveInPsd();
    void guidePreferences();
    void unitsPreferences();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void RulersGuidesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void RulersGuidesTest::rulerScale()
{
    using pictura::CanvasRuler;
    using pictura::RulerUnit;
    const auto check = [](RulerUnit unit, double screenPerUnit, double major, int subdivisions) {
        const CanvasRuler::Scale scale = CanvasRuler::scaleFor(unit, screenPerUnit);
        return scale.major == major && scale.subdivisions == subdivisions;
    };
    // Pixels: 1 screen px per px at 100 %.
    QVERIFY(check(RulerUnit::Pixels, 1.0, 50.0, 5));
    QVERIFY(check(RulerUnit::Pixels, 0.1, 500.0, 5));
    QVERIFY(check(RulerUnit::Pixels, 8.0, 10.0, 10));
    QVERIFY2(check(RulerUnit::Pixels, 32.0, 2.0, 2), "never a tick between pixels");
    // Inches at 72 ppi: labelled every inch in eighths at 100 %, as in CS6.
    QVERIFY(check(RulerUnit::Inches, 72.0, 1.0, 8));
    QVERIFY(check(RulerUnit::Inches, 18.0, 5.0, 5));
    QVERIFY(check(RulerUnit::Inches, 72.0 * 16, 0.05, 5));
    QVERIFY(check(RulerUnit::Centimeters, 72.0 / 2.54, 2.0, 4));
    for (const double screen : {0.01, 0.37, 1.0, 3.3, 32.0, 900.0}) {
        for (int unit = 0; unit < pictura::kRulerUnitCount; ++unit) {
            const CanvasRuler::Scale scale = CanvasRuler::scaleFor(RulerUnit(unit), screen);
            QVERIFY(scale.major * screen >= 50.0);
            QVERIFY(scale.subdivisions == 1
                    || scale.major / scale.subdivisions * screen >= 4.0);
        }
    }
}

void RulersGuidesTest::rulersToggle()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_rulers_toggle"));
    QVERIFY(f.ok());
    pictura::CanvasScrollBars* host = hostOf(f);
    QVERIFY(host != nullptr);
    QAction* rulers = action(*window_, pictura::command_ids::ViewRulers);
    QVERIFY(rulers->isCheckable());
    QVERIFY2(!rulers->isChecked(), "rulers default off");
    QVERIFY(!host->horizontalRuler()->isVisible());

    rulers->trigger();
    QVERIFY(host->horizontalRuler()->isVisible());
    QVERIFY(host->verticalRuler()->isVisible());
    QVERIFY(pictura::loadSession().rulersVisible);
    // The canvas gives up the rulers' room.
    QCOMPARE(f.canvas->mapTo(host, QPoint(0, 0)),
             QPoint(pictura::CanvasRuler::kThickness, pictura::CanvasRuler::kThickness));

    // Inches by default, CS6's unit, at the document's resolution; the
    // context menu's pick applies to both rulers and persists.
    pictura::CanvasRuler* top = host->horizontalRuler();
    QCOMPARE(top->unit(), pictura::RulerUnit::Inches);
    QCOMPARE(top->pixelsPerUnit(), 72.0);
    emit top->unitChosen(pictura::RulerUnit::Percent);
    QCOMPARE(host->verticalRuler()->unit(), pictura::RulerUnit::Percent);
    QCOMPARE(top->pixelsPerUnit(), 0.8);
    QCOMPARE(host->verticalRuler()->pixelsPerUnit(), 0.6);
    QCOMPARE(pictura::loadSession().rulerUnit, int(pictura::RulerUnit::Percent));
    emit top->unitChosen(pictura::RulerUnit::Inches);

    rulers->trigger();
    QVERIFY(!host->horizontalRuler()->isVisible());
    QVERIFY(!pictura::loadSession().rulersVisible);
}

void RulersGuidesTest::togglesDuringLoadingDoNotCrash()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString psd = dir.filePath(QStringLiteral("loading.psd"));
    QVERIFY(window_->newDocument(QStringLiteral("Loading"), 64, 48, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY(window_->saveActiveAs(psd));
    QVERIFY(window_->closeDocument(0, false));

    window_->setOpenDelayForTest(250);
    QVERIFY(window_->startOpenPath(psd) != 0);
    QCOMPARE(window_->documentCount(), 1);
    QVERIFY2(window_->isLoadingDocument(0), "the tab is loading");

    QAction* rulers = action(*window_, pictura::command_ids::ViewRulers);
    QAction* guides = action(*window_, pictura::command_ids::ViewShowGuides);
    QVERIFY(rulers && guides);
    {
        // Restore the suite's starting state on every exit path: the toggles
        // below flip both flags, and later tests assume rulers off, guides on.
        struct RestoreToggles {
            QAction* rulers;
            QAction* guides;
            ~RestoreToggles()
            {
                rulers->trigger();
                guides->trigger();
            }
        } restore{rulers, guides};
        rulers->trigger();
        guides->trigger();

        window_->waitForOpensForTest();
        window_->setOpenDelayForTest(0);
        QVERIFY(!window_->isLoadingDocument(0));

        pictura::ImageView* canvas = window_->canvasAt(0);
        QVERIFY(canvas);
        QVERIFY2(!canvas->guidesVisible(), "the guide toggle reached the completed canvas");
        pictura::CanvasScrollBars* host = nullptr;
        for (auto* candidate : window_->findChildren<pictura::CanvasScrollBars*>()) {
            if (candidate->view() == canvas) {
                host = candidate;
            }
        }
        QVERIFY(host);
        QVERIFY2(host->horizontalRuler()->isVisibleTo(window_.get()),
                 "the ruler toggle reached the completed canvas");
    }
    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(0, false));
    }
}

void RulersGuidesTest::dragGuideFromRuler()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_rulers_drag"));
    QVERIFY(f.ok());
    action(*window_, pictura::command_ids::ViewRulers)->trigger();
    pictura::CanvasScrollBars* host = hostOf(f);
    QVERIFY(host != nullptr);
    QCoreApplication::processEvents();
    const int base = f.view->history_index();

    // Top ruler -> a horizontal guide at the document row under the release.
    const QPoint at = f.canvas->imageToWidget(QPointF(10, 20)).toPoint();
    const double row = std::round(f.canvas->widgetToImage(QPointF(at)).y());
    QVERIFY(std::abs(row - 20.0) <= 1.0);
    dragFromRuler(host->horizontalRuler(), QPoint(30, 8), f.canvas->mapToGlobal(at));
    QCOMPARE(pictura::guide_count(*f.view), 1);
    QCOMPARE(guide(f.view, 0), QList<double>({0.0, row}));
    QVERIFY(f.committedOnce(base, "New Guide"));
    QCOMPARE(f.canvas->guides().size(), 1);
    QVERIFY(!f.canvas->hasGuidePreviewForTest());

    QVERIFY(f.view->undo());
    QCOMPARE(pictura::guide_count(*f.view), 0);
    QVERIFY(f.canvas->guides().isEmpty());

    // Left ruler, released back over the ruler: nothing.
    pictura::CanvasRuler* left = host->verticalRuler();
    dragFromRuler(left, QPoint(8, 30), left->mapToGlobal(QPoint(8, 40)));
    QCOMPARE(pictura::guide_count(*f.view), 0);
    QCOMPARE(f.view->history_index(), base);

    // Left ruler onto the canvas: a vertical guide.
    const QPoint columnAt = f.canvas->imageToWidget(QPointF(33, 5)).toPoint();
    const double column = std::round(f.canvas->widgetToImage(QPointF(columnAt)).x());
    dragFromRuler(left, QPoint(8, 30), f.canvas->mapToGlobal(columnAt));
    QCOMPARE(guide(f.view, 0), QList<double>({1.0, column}));
    action(*window_, pictura::command_ids::ViewRulers)->trigger();
}

void RulersGuidesTest::moveAndDeleteGuide()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_guides_move"));
    QVERIFY(f.ok());
    QCOMPARE(pictura::add_guide(*f.view, true, 30.0), 0);
    window_->setActiveTool(pictura::ToolId::Move);
    const int base = f.view->history_index();

    // Within a few screen pixels of the guide, the Move tool picks it up.
    const double z = f.canvas->zoom();
    f.drag({QPointF(30 + 2 / z, 10), QPointF(40, 12), QPointF(45.3, 12)});
    QCOMPARE(guide(f.view, 0), QList<double>({1.0, 45.0}));
    QVERIFY(f.committedOnce(base, "Move Guide"));
    QCOMPARE(f.canvas->guides().first().position, 45.0);

    // A click that leaves it in place records nothing.
    f.drag({QPointF(45, 10), QPointF(45, 10)});
    QCOMPARE(f.view->history_index(), base + 1);

    // Dragged off the canvas, it is deleted.
    const QPointF outside = f.canvas->widgetToImage(QPointF(-20, 10));
    f.drag({QPointF(45, 10), outside});
    QCOMPARE(pictura::guide_count(*f.view), 0);
    QVERIFY(f.committedOnce(base + 1, "Delete Guide"));

    // Another tool reaches a guide only with Ctrl held.
    pictura::add_guide(*f.view, false, 25.0);
    window_->setActiveTool(pictura::ToolId::Brush);
    const int painted = f.view->history_index();
    f.drag({QPointF(5, 25), QPointF(5, 35)}, Qt::ControlModifier);
    QCOMPARE(guide(f.view, 0), QList<double>({0.0, 35.0}));
    QVERIFY(f.committedOnce(painted, "Move Guide"));

    // The Hand tool pans on its own press, but Ctrl still reaches a guide, and
    // a Ctrl press that misses one pans.
    window_->setActiveTool(pictura::ToolId::Hand);
    QVERIFY(f.canvas->panEnabled());
    const int handBase = f.view->history_index();
    dragOnCanvas(f.canvas, QPointF(5, 35), QPointF(5, 20), Qt::ControlModifier);
    QCOMPARE(guide(f.view, 0), QList<double>({0.0, 20.0}));
    QVERIFY(f.committedOnce(handBase, "Move Guide"));
    const QPointF before = f.canvas->offset();
    dragOnCanvas(f.canvas, QPointF(40, 45), QPointF(30, 50), Qt::ControlModifier);
    QVERIFY(f.canvas->offset() != before);
    QCOMPARE(guide(f.view, 0), QList<double>({0.0, 20.0}));
    QCOMPARE(f.view->history_index(), handBase + 1);
}

void RulersGuidesTest::lockedGuides()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_guides_lock"));
    QVERIFY(f.ok());
    pictura::add_guide(*f.view, true, 30.0);
    window_->setActiveTool(pictura::ToolId::Move);
    QAction* lock = action(*window_, pictura::command_ids::ViewLockGuides);
    lock->trigger();
    QVERIFY(pictura::loadSession().guidesLocked);
    const int base = f.view->history_index();
    f.drag({QPointF(30, 10), QPointF(50, 10)});
    QVERIFY(!f.tools->guideDragActiveForTest());
    QCOMPARE(guide(f.view, 0), QList<double>({1.0, 30.0}));
    for (int i = base + 1; i <= f.view->history_index(); ++i) {
        QVERIFY(!f.view->history_label(i).contains(QStringLiteral("Guide")));
    }
    lock->trigger();

    // Hidden guides are not picked up either.
    QAction* show = action(*window_, pictura::command_ids::ViewShowGuides);
    QVERIFY2(show->isChecked(), "guides shown by default");
    show->trigger();
    QVERIFY(!f.canvas->guidesVisible());
    const int hiddenBase = f.view->history_index();
    f.drag({QPointF(30, 10), QPointF(50, 10)});
    QCOMPARE(guide(f.view, 0), QList<double>({1.0, 30.0}));
    for (int i = hiddenBase + 1; i <= f.view->history_index(); ++i) {
        QVERIFY(!f.view->history_label(i).contains(QStringLiteral("Guide")));
    }
    show->trigger();
    QVERIFY(f.canvas->guidesVisible());
}

void RulersGuidesTest::clearAndNewGuide()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_guides_clear"));
    QVERIFY(f.ok());
    QAction* clear = action(*window_, pictura::command_ids::ViewClearGuides);
    QVERIFY2(!clear->isEnabled(), "nothing to clear");
    pictura::add_guide(*f.view, true, 10.0);
    pictura::add_guide(*f.view, false, 20.0);
    const int base = f.view->history_index();
    clear = action(*window_, pictura::command_ids::ViewClearGuides);
    QVERIFY(clear->isEnabled());
    clear->trigger();
    QCOMPARE(pictura::guide_count(*f.view), 0);
    QVERIFY(f.committedOnce(base, "Clear Guides"));
    QVERIFY(!action(*window_, pictura::command_ids::ViewClearGuides)->isEnabled());

    QTimer::singleShot(0, window_.get(), [this]() {
        auto* dialog = window_->findChild<pictura::NewGuideDialog*>();
        QVERIFY(dialog != nullptr);
        dialog->setGuide(true, 12.0);
        dialog->accept();
    });
    action(*window_, pictura::command_ids::ViewNewGuide)->trigger();
    QCOMPARE(guide(f.view, 0), QList<double>({1.0, 12.0}));
    QVERIFY(f.committedOnce(base + 1, "New Guide"));
}

void RulersGuidesTest::guidesSaveInPsd()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_guides_psd"));
    QVERIFY(f.ok());
    pictura::add_guide(*f.view, true, 40.0);
    pictura::add_guide(*f.view, false, -6.0);
    QTemporaryDir dir;
    const QString path = dir.filePath(QStringLiteral("guides.psd"));
    QVERIFY(f.view->save(path));

    pictura::PictureView reopened;
    QVERIFY(reopened.open(path));
    QCOMPARE(pictura::guide_count(reopened), 2);
    QCOMPARE(guide(&reopened, 0), QList<double>({1.0, 40.0}));
    QCOMPARE(guide(&reopened, 1), QList<double>({0.0, -6.0}));
}

void RulersGuidesTest::guidePreferences()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_guides_prefs"));
    QVERIFY(f.ok());
    QCOMPARE(f.canvas->guideColor(), QColor(0x4a, 0xff, 0xff));
    QVERIFY(!f.canvas->guidesDashed());

    action(*window_, pictura::command_ids::EditPreferencesGuides)->trigger();
    pictura::PreferencesDialog* prefs = window_->preferencesDialog();
    QVERIFY(prefs != nullptr);
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kGuides);
    auto* color = prefs->findChild<QComboBox*>(QStringLiteral("preferencesGuideColor"));
    auto* style = prefs->findChild<QComboBox*>(QStringLiteral("preferencesGuideStyle"));
    QVERIFY(color && style);
    QCOMPARE(color->currentText(), QStringLiteral("Cyan"));
    QCOMPARE(style->currentText(), QStringLiteral("Lines"));

    const int magenta = color->findText(QStringLiteral("Magenta"));
    color->setCurrentIndex(magenta);
    emit color->activated(magenta);
    style->setCurrentIndex(1);
    emit style->activated(1);
    QCOMPARE(f.canvas->guideColor(), QColor(0xff, 0x4a, 0xff));
    QVERIFY(f.canvas->guidesDashed());
    const pictura::SessionState state = pictura::loadSession();
    QCOMPARE(state.guideColor, QStringLiteral("#ff4aff"));
    QVERIFY(state.guideDashed);
    prefs->close();
}

// Edit > Preferences > Units & Rulers (#299): the Rulers unit and Point/Pica
// Size drive every ruler, persist, and stay in sync with a ruler's context
// menu; a double-click on a ruler opens the page.
void RulersGuidesTest::unitsPreferences()
{
    Fixture f(*window_, seedImage(), QStringLiteral("pictura_units_prefs"));
    QVERIFY(f.ok());
    pictura::CanvasScrollBars* host = hostOf(f);
    QVERIFY(host != nullptr);
    pictura::CanvasRuler* top = host->horizontalRuler();

    action(*window_, pictura::command_ids::EditPreferencesUnits)->trigger();
    pictura::PreferencesDialog* prefs = window_->preferencesDialog();
    QVERIFY(prefs != nullptr);
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kUnits);
    auto* units = prefs->findChild<QComboBox*>(QStringLiteral("preferencesRulerUnits"));
    auto* traditional =
        prefs->findChild<QRadioButton*>(QStringLiteral("preferencesTraditionalPoints"));
    QVERIFY(units && traditional);
    QCOMPARE(units->count(), pictura::kRulerUnitCount);
    QCOMPARE(units->currentText(), QStringLiteral("Inches"));

    const int points = units->findText(QStringLiteral("Points"));
    units->setCurrentIndex(points);
    emit units->activated(points);
    QCOMPARE(top->unit(), pictura::RulerUnit::Points);
    QCOMPARE(host->verticalRuler()->unit(), pictura::RulerUnit::Points);
    QCOMPARE(top->pixelsPerUnit(), 1.0);
    QCOMPARE(pictura::loadSession().rulerUnit, int(pictura::RulerUnit::Points));

    traditional->setChecked(true);
    QVERIFY(std::abs(top->pixelsPerUnit() - 72.0 / 72.27) < 1e-12);
    QVERIFY(pictura::loadSession().traditionalPoints);
    prefs->findChild<QRadioButton*>(QStringLiteral("preferencesPostScriptPoints"))
        ->setChecked(true);
    QCOMPARE(top->pixelsPerUnit(), 1.0);

    // The context menu's pick shows on the page.
    emit top->unitChosen(pictura::RulerUnit::Centimeters);
    QCOMPARE(units->currentText(), QStringLiteral("Centimeters"));
    prefs->close();

    // A double-click on a ruler opens the page.
    prefs->openOn(pictura::PreferencesDialog::kGeneral);
    prefs->close();
    QTest::mouseDClick(top, Qt::LeftButton, Qt::NoModifier, QPoint(20, 8));
    QVERIFY(prefs->isVisible());
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kUnits);
    prefs->close();
    emit top->unitChosen(pictura::RulerUnit::Inches);
}

QTEST_MAIN(RulersGuidesTest)
#include "tst_rulers_guides.moc"
