#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtGui/QAction>
#include <QtGui/QImage>

#include "api/layers_api.h"
#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

// Migrated from the hand-rolled self-test check `pictura_raw` (exit 528).
class PicturaRawTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rasterLayerFiltersDestructively();
    void backgroundFiltersDestructively();
    void smartObjectAttachesSmartFilter();
    void ineligibleTargetRecordsNothing();
    void menuEnablement();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PicturaRawTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void PicturaRawTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

void PicturaRawTest::rasterLayerFiltersDestructively()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawDestructive"), 8, 8,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));

    const int base = view->history_count();
    const unsigned int before = view->sample_argb(0, 0);
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));

    // Destructive: pixels change, no smart object, no retained settings.
    const QString state = view->layer_smart_object_state(path);
    QVERIFY2(!state.startsWith(QStringLiteral("embedded:")), qPrintable(state));
    QVERIFY(view->sample_argb(0, 0) != before);
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));
    QVERIFY(view->layer_pictura_raw_settings(path).isEmpty());
}

void PicturaRawTest::backgroundFiltersDestructively()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawBg"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    // The new document's sole layer becomes the Background, so the composite
    // below reflects the filtered pixels.
    const QString bg = pictura::layers_api::rowAt(*view, 0).path;
    QVERIFY(!bg.isEmpty());
    QVERIFY(view->background_from_layer(bg));
    QCOMPARE(pictura::layers_api::rowAt(*view, 0).kind, QStringLiteral("background"));

    const int base = view->history_count();
    const unsigned int before = view->sample_argb(0, 0);
    QVERIFY(view->apply_pictura_raw_filter(bg, QStringLiteral("Temp=20;Ex12=-1.5")));
    QVERIFY(view->sample_argb(0, 0) != before);
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));
    QVERIFY(view->layer_pictura_raw_settings(bg).isEmpty());
    QCOMPARE(pictura::layers_api::rowAt(*view, 0).kind, QStringLiteral("background"));
}

void PicturaRawTest::smartObjectAttachesSmartFilter()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawSmart"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->layer_can_convert_to_smart_object(path));
    QVERIFY(view->convert_to_smart_object(path));

    const int base = view->history_count();
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));

    // Non-destructive: stays a smart object with the camera-raw filter attached.
    const QString state = view->layer_smart_object_state(path);
    QVERIFY2(state.startsWith(QStringLiteral("embedded:")), qPrintable(state));
    const QString stored = view->layer_pictura_raw_settings(path);
    QVERIFY2(stored.contains(QStringLiteral("Temp=20")), qPrintable(stored));
    QVERIFY2(stored.contains(QStringLiteral("Ex12=1.5")), qPrintable(stored));

    QVERIFY(!view->pictura_raw_preview(path, QStringLiteral("Temp=20;Ex12=1.5"), 256).isNull());
}

void PicturaRawTest::ineligibleTargetRecordsNothing()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawRefuse"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    const QString group = view->add_group_in(QString());
    QVERIFY(!view->layer_can_apply_pictura_raw(group));
    const int base = view->history_count();
    QVERIFY(!view->apply_pictura_raw_filter(group, QStringLiteral("Temp=10")));
    QCOMPARE(view->history_count(), base);
    QVERIFY(view->pictura_raw_preview(group, QStringLiteral("Temp=10"), 256).isNull());
}

void PicturaRawTest::menuEnablement()
{
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QAction* raw = registry->action(QString::fromLatin1(pictura::command_ids::FilterPicturaRaw));
    QVERIFY(raw != nullptr);

    // No document open: disabled.
    registry->refresh();
    QVERIFY(!raw->isEnabled());

    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawMenu"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->layer_can_apply_pictura_raw(path));

    panel->setView(view);
    panel->refresh();
    QVERIFY(panel->selectRowForTest(path));
    QCoreApplication::processEvents();
    registry->refresh();
    QVERIFY2(raw->isEnabled(), "Pictura Raw is enabled for a raster layer");
}

QTEST_MAIN(PicturaRawTest)
#include "tst_pictura_raw.moc"
