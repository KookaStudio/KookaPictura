// Edit > Auto-Align Layers: menu availability, the dialog's defaults and
// remembered choice, one history state with undo, cancellation, a stale run,
// a missing lens profile, and a real Reposition Only commit. The matcher
// itself is tested in pictura-render.

#include <QtTest/QtTest>

#include "auto_align_dialog.h"
#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"

#include "api/filters_bridge.h"
#include "api/transform_api.h"
#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QSet>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QRadioButton>

#include "qt_test_support.h"

class AutoAlignTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuEnablement();
    void dialogDefaults();
    void dialogRestoresAChoice();
    void lensSubOptionsFollowProjection();
    void repositionOnlyMovesLayers();
    void oneHistoryStateAndUndo();
    void cancelMidRun();
    void staleRunIsDropped();
    void missingProfileIsReported();
    void selectionAfterCommit();

private:
    bool setupLayers();
    QString seedPath() const;
    QAction* leaf() const;
    void select(const QStringList& paths);
    void settle();

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QStringList paths_;
};

void AutoAlignTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void AutoAlignTest::cleanup()
{
    if (view_) {
        auto_align_set_delay_for_test(*view_, 0);
    }
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
    paths_.clear();
}

QString AutoAlignTest::seedPath() const
{
    return dir_.filePath(QStringLiteral("auto-align-seed.png"));
}

// A deterministic high-contrast texture, opened as the document's source layer.
bool AutoAlignTest::setupLayers()
{
    QImage seed(96, 72, QImage::Format_RGB32);
    seed.fill(Qt::white);
    {
        QPainter painter(&seed);
        for (int y = 0; y < 72; y += 6) {
            for (int x = 0; x < 96; x += 6) {
                const int v = (x * 31 + y * 17) % 200;
                painter.fillRect(x + 1, y + 1, 4, 4,
                                 QColor(v, (v * 3) % 255, (v * 7) % 255));
            }
        }
    }
    if (!seed.save(seedPath(), "PNG")) {
        return false;
    }
    if (!window_->newDocument(QStringLiteral("Auto-Align"), 96, 72, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_ || !view_->open_image(seedPath())) {
        return false;
    }
    // Two copies of overlapping regions of the source, offset by (6, 4).
    view_->select_rect(10, 10, 50, 40, QStringLiteral("new"), 0.0);
    const QString a = view_->layer_via_copy(QStringLiteral("0"));
    view_->deselect();
    view_->select_rect(10, 10, 50, 40, QStringLiteral("new"), 0.0);
    const QString b = view_->layer_via_copy(QStringLiteral("0"));
    view_->deselect();
    if (a.isEmpty() || b.isEmpty()) {
        return false;
    }
    // The second copy lands above the first, so the first copy's index shifted.
    paths_ = {QString::number(1), QString::number(2)};
    // Move the top copy by (6, 4) so the two copies are offset copies of the
    // same content, which is what Auto-Align can put back together.
    view_->set_active_layer(paths_[0]);
    window_->setActiveTool(pictura::ToolId::Move);
    const auto nudge = [this](int key, int times) {
        for (int i = 0; i < times; ++i) {
            QKeyEvent event(QEvent::KeyPress, key, Qt::NoModifier);
            QApplication::sendEvent(window_.get(), &event);
        }
    };
    nudge(Qt::Key_Right, 6);
    nudge(Qt::Key_Down, 4);
    QCoreApplication::processEvents();
    return true;
}

QAction* AutoAlignTest::leaf() const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Edit"), QStringLiteral("Auto-Align Layers…")}));
}

void AutoAlignTest::select(const QStringList& paths)
{
    panel_->selectPaths(paths, paths.first());
    window_->registry()->refresh();
}

void AutoAlignTest::settle()
{
    QTRY_VERIFY(!auto_align_running(*view_));
}

void AutoAlignTest::menuEnablement()
{
    QVERIFY(setupLayers());
    QAction* action = leaf();
    QVERIFY(action);
    select({paths_[0]});
    QVERIFY(!action->isEnabled());
    select(paths_);
    QVERIFY(action->isEnabled());

    select(paths_);
    view_->select_rect(20, 20, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(view_->begin_transform_mode(paths_[0], QStringLiteral("skew")));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    view_->cancel_transform();
    view_->deselect();
    QCoreApplication::processEvents();
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
}

void AutoAlignTest::dialogDefaults()
{
    pictura::AutoAlignDialog dialog;
    QVERIFY(dialog.findChild<QRadioButton*>(QStringLiteral("autoAlignAuto"))->isChecked());
    QCOMPARE(dialog.projection(), 0);
    QVERIFY(!dialog.vignette());
    QVERIFY(!dialog.geometric());
    auto* vignette = dialog.findChild<QCheckBox*>(QStringLiteral("autoAlignVignette"));
    auto* geometric = dialog.findChild<QCheckBox*>(QStringLiteral("autoAlignGeometric"));
    QVERIFY(vignette && geometric);
    QVERIFY(!vignette->isEnabled());
    QVERIFY(!geometric->isEnabled());
}

void AutoAlignTest::dialogRestoresAChoice()
{
    pictura::AutoAlignDialog first;
    first.findChild<QRadioButton*>(QStringLiteral("autoAlignLens"))->setChecked(true);
    first.findChild<QCheckBox*>(QStringLiteral("autoAlignVignette"))->setChecked(true);
    first.findChild<QCheckBox*>(QStringLiteral("autoAlignGeometric"))->setChecked(true);
    QCOMPARE(first.projection(), 6);
    QVERIFY(first.vignette());
    QVERIFY(first.geometric());

    pictura::AutoAlignDialog next(nullptr, first.choice());
    QCOMPARE(next.projection(), 6);
    QVERIFY(next.findChild<QRadioButton*>(QStringLiteral("autoAlignLens"))->isChecked());
    QVERIFY(next.vignette());
    QVERIFY(next.geometric());
    QVERIFY(next.findChild<QCheckBox*>(QStringLiteral("autoAlignVignette"))->isEnabled());

    // A choice survives a projection where the options are unavailable.
    pictura::AutoAlignDialog autoDialog(nullptr, {0, true, true});
    QVERIFY(!autoDialog.vignette());
    QVERIFY(autoDialog.choice().vignette);
    QVERIFY(autoDialog.choice().geometric);
}

void AutoAlignTest::lensSubOptionsFollowProjection()
{
    pictura::AutoAlignDialog dialog;
    auto* vignette = dialog.findChild<QCheckBox*>(QStringLiteral("autoAlignVignette"));
    auto* geometric = dialog.findChild<QCheckBox*>(QStringLiteral("autoAlignGeometric"));
    QVERIFY(vignette && geometric);
    QVERIFY(!vignette->isEnabled());
    dialog.findChild<QRadioButton*>(QStringLiteral("autoAlignLens"))->setChecked(true);
    QVERIFY(vignette->isEnabled());
    QVERIFY(geometric->isEnabled());
    vignette->setChecked(true);
    QVERIFY(dialog.vignette());
    dialog.findChild<QRadioButton*>(QStringLiteral("autoAlignPerspective"))->setChecked(true);
    QVERIFY(!vignette->isEnabled());
    QVERIFY(!dialog.vignette());
}

void AutoAlignTest::repositionOnlyMovesLayers()
{
    QVERIFY(setupLayers());
    select(paths_);
    QVERIFY(view_->set_layers_lock(QStringList{paths_[1]}, QStringLiteral("position"), true));
    const auto reference = pictura::transform_api::contentRect(*view_, paths_[1]);
    const auto before = pictura::transform_api::contentRect(*view_, paths_[0]);
    QVERIFY(reference.valid && before.valid);
    QVERIFY(qAbs(before.left - reference.left) > 2 || qAbs(before.top - reference.top) > 2);
    QVERIFY(pictura::runAutoAlign(*window_, 5, false, false));
    QCOMPARE(pictura::transform_api::contentRect(*view_, paths_[1]).toString(),
             reference.toString());
    const auto after = pictura::transform_api::contentRect(*view_, paths_[0]);
    QVERIFY2(qAbs(after.left - reference.left) <= 2 && qAbs(after.top - reference.top) <= 2
                 && qAbs(after.right - reference.right) <= 2
                 && qAbs(after.bottom - reference.bottom) <= 2,
             qPrintable(QStringLiteral("aligned %1 vs %2").arg(after.toString(),
                                                              reference.toString())));
}

void AutoAlignTest::oneHistoryStateAndUndo()
{
    QVERIFY(setupLayers());
    select(paths_);
    QVERIFY(view_->set_layers_lock(QStringList{paths_[1]}, QStringLiteral("position"), true));
    const auto before = pictura::transform_api::contentRect(*view_, paths_[0]);
    const int history = view_->history_count();
    QVERIFY(pictura::runAutoAlign(*window_, 5, false, false));
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Auto-Align Layers"));
    QVERIFY(view_->undo());
    QCOMPARE(pictura::transform_api::contentRect(*view_, paths_[0]).toString(),
             before.toString());
}

void AutoAlignTest::cancelMidRun()
{
    QVERIFY(setupLayers());
    select(paths_);
    const int history = view_->history_count();
    const uint before = view_->composite_argb(24, 24);
    auto_align_set_delay_for_test(*view_, 60000);
    QVERIFY(auto_align_start(*view_, paths_, 5, false, false));
    QVERIFY(auto_align_running(*view_));
    auto_align_cancel(*view_);
    settle();
    QVERIFY(auto_align_finish(*view_).isEmpty());
    QCOMPARE(view_->history_count(), history);
    QCOMPARE(view_->composite_argb(24, 24), before);
}

void AutoAlignTest::staleRunIsDropped()
{
    QVERIFY(setupLayers());
    select(paths_);
    auto_align_set_delay_for_test(*view_, 200);
    QVERIFY(auto_align_start(*view_, paths_, 5, false, false));
    QVERIFY(!view_->add_layer_in(paths_.last()).isEmpty());
    const int history = view_->history_count();
    settle();
    QVERIFY(auto_align_finish(*view_).isEmpty());
    QVERIFY(!auto_align_notice(*view_).isEmpty());
    QCOMPARE(view_->history_count(), history);
}

void AutoAlignTest::missingProfileIsReported()
{
    QVERIFY(setupLayers());
    select(paths_);
    // A PNG carries no EXIF, so no lens profile matches.
    QVERIFY(pictura::runAutoAlign(*window_, 6, true, true));
    const QString notice = auto_align_notice(*view_);
    QVERIFY2(notice.contains(QStringLiteral("lens profile"), Qt::CaseInsensitive),
             qPrintable(notice));
}

void AutoAlignTest::selectionAfterCommit()
{
    QVERIFY(setupLayers());
    select(paths_);
    QVERIFY(pictura::runAutoAlign(*window_, 5, false, false));
    const QStringList paths = panel_->selectedPaths();
    const QSet<QString> selected(paths.begin(), paths.end());
    QVERIFY(selected.contains(paths_[0]));
    QVERIFY(selected.contains(paths_[1]));
}

QTEST_MAIN(AutoAlignTest)
#include "tst_auto_align.moc"
