// Edit > Auto-Blend Layers: availability, the dialog's defaults, one history
// state with one undo, cancelling, a document edited mid-run, and the selection
// after a commit. The blend itself is tested in pictura-render.

#include <QtTest/QtTest>

#include "auto_blend_dialog.h"
#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/auto_blend.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"

#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>

#include "qt_test_support.h"

class AutoBlendTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuEnablement();
    void refusedModes();
    void dialogDefaults();
    void fillFollowsDepth();
    void oneHistoryStateAndUndo();
    void cancelMidRun();
    void staleRunIsDropped();
    void selectionAfterCommit();

private:
    bool setupLayers();
    QString rasterLayer(uint rgba);
    void select(const QStringList& paths);
    QAction* leaf() const;
    void settle();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QStringList paths_;
};

void AutoBlendTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void AutoBlendTest::cleanup()
{
    if (view_) {
        auto_blend_set_delay_for_test(*view_, 0);
    }
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

QString AutoBlendTest::rasterLayer(uint rgba)
{
    const QString path = view_->add_solid_fill(rgba);
    return !path.isEmpty() && view_->rasterize_fill_content(path) ? path : QString();
}

// Two opaque rasters of different tones over a Background, so Seamless corrects.
bool AutoBlendTest::setupLayers()
{
    if (!window_->newDocument(QStringLiteral("Blend"), 24, 24, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_) {
        return false;
    }
    const QString lower = rasterLayer(0xff505050u);
    const QString upper = rasterLayer(0xffb0b0b0u);
    paths_ = {lower, upper};
    QCoreApplication::processEvents();
    return !lower.isEmpty() && !upper.isEmpty();
}

void AutoBlendTest::select(const QStringList& paths)
{
    panel_->selectPaths(paths, paths.first());
    QCoreApplication::processEvents();
}

QAction* AutoBlendTest::leaf() const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Edit"), QStringLiteral("Auto-Blend Layers…")}));
}

void AutoBlendTest::settle()
{
    QTRY_VERIFY(!auto_blend_running(*view_));
}

void AutoBlendTest::menuEnablement()
{
    QVERIFY(setupLayers());
    QAction* action = leaf();
    QVERIFY(action);
    select({paths_.first()});
    QVERIFY(!action->isEnabled());
    select(paths_);
    QVERIFY(action->isEnabled());
    QVERIFY(view_->background_from_layer(QStringLiteral("0")));
    QCoreApplication::processEvents();
    select({QStringLiteral("0"), paths_.first()});
    QVERIFY2(!action->isEnabled(), "Background selected");
}

void AutoBlendTest::refusedModes()
{
    QVERIFY(setupLayers());
    select(paths_);
    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("lab")));
    QVERIFY(!leaf()->isEnabled());
    const int history = view_->history_count();
    QVERIFY(!pictura::runAutoBlend(*window_, false, false));
    QCOMPARE(view_->history_count(), history);
}

void AutoBlendTest::dialogDefaults()
{
    pictura::AutoBlendDialog dialog;
    QVERIFY(dialog.findChild<QRadioButton*>(QStringLiteral("autoBlendPanorama"))->isChecked());
    QVERIFY(!dialog.stackImages());
    QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("autoBlendSeamless"))->isChecked());
    QVERIFY(!dialog.seamless());
    QVERIFY(!dialog.fillTransparent());
    for (const char* name : {"autoBlendPanoramaPreview", "autoBlendStackPreview"}) {
        const auto* preview = dialog.findChild<QLabel*>(QLatin1String(name));
        QVERIFY(preview && !preview->pixmap().isNull());
    }
}

void AutoBlendTest::fillFollowsDepth()
{
    pictura::AutoBlendDialog eight(true);
    QVERIFY(eight.findChild<QCheckBox*>(QStringLiteral("autoBlendFill"))->isEnabled());
    pictura::AutoBlendDialog sixteen(false);
    auto* fill = sixteen.findChild<QCheckBox*>(QStringLiteral("autoBlendFill"));
    QVERIFY(!fill->isEnabled());
    fill->setChecked(true);
    QVERIFY(!sixteen.fillTransparent());
}

void AutoBlendTest::oneHistoryStateAndUndo()
{
    QVERIFY(setupLayers());
    select(paths_);
    const uint before = view_->composite_argb(12, 12);
    const int layers = view_->layer_count();
    const int history = view_->history_count();
    QVERIFY(pictura::runAutoBlend(*window_, false, true));
    QVERIFY(view_->layer_count() > layers);
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Auto-Blend Layers"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(12, 12), before);
    QCOMPARE(view_->layer_count(), layers);
}

void AutoBlendTest::cancelMidRun()
{
    QVERIFY(setupLayers());
    const int history = view_->history_count();
    const uint before = view_->composite_argb(12, 12);
    auto_blend_set_delay_for_test(*view_, 60000);
    QVERIFY(auto_blend_start(*view_, paths_, false, true, false));
    QVERIFY(auto_blend_running(*view_));
    auto_blend_cancel(*view_);
    settle();
    QVERIFY(auto_blend_finish(*view_).isEmpty());
    QVERIFY(auto_blend_error(*view_).isEmpty());
    QCOMPARE(view_->history_count(), history);
    QCOMPARE(view_->composite_argb(12, 12), before);
}

void AutoBlendTest::staleRunIsDropped()
{
    QVERIFY(setupLayers());
    auto_blend_set_delay_for_test(*view_, 200);
    QVERIFY(auto_blend_start(*view_, paths_, false, false, false));
    QVERIFY(!view_->add_layer_in(paths_.last()).isEmpty());
    const int history = view_->history_count();
    settle();
    QVERIFY(auto_blend_finish(*view_).isEmpty());
    QVERIFY(!auto_blend_error(*view_).isEmpty());
    QCOMPARE(view_->history_count(), history);
}

void AutoBlendTest::selectionAfterCommit()
{
    QVERIFY(setupLayers());
    select(paths_);
    QVERIFY(pictura::runAutoBlend(*window_, false, true));
    QCoreApplication::processEvents();
    QStringList selected = window_->selectedLayerPaths();
    selected.sort();
    QCOMPARE(selected, paths_);
}

QTEST_MAIN(AutoBlendTest)
#include "tst_auto_blend.moc"
