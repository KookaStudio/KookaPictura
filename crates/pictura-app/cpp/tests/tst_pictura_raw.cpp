#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtGui/QImage>

#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

// Migrated from the hand-rolled self-test check `pictura_raw` (exit 528).
class PicturaRawTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void applyConvertsAndRecordsOneState();
    void reapplyRecordsOneMoreState();
    void ineligibleTargetRecordsNothing();

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

void PicturaRawTest::applyConvertsAndRecordsOneState()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawCtl"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->layer_can_convert_to_smart_object(path));

    const int base = view->history_count();
    const unsigned int before = view->sample_argb(0, 0);
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));

    const QString state = view->layer_smart_object_state(path);
    QVERIFY2(state.startsWith(QStringLiteral("embedded:")), qPrintable(state));
    QVERIFY(view->sample_argb(0, 0) != before);
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));

    const QString stored = view->layer_pictura_raw_settings(path);
    QVERIFY(!stored.isEmpty());
    QVERIFY2(stored.contains(QStringLiteral("Temp=20")), qPrintable(stored));
    QVERIFY2(stored.contains(QStringLiteral("Ex12=1.5")), qPrintable(stored));

    QVERIFY(!view->pictura_raw_preview(path, QStringLiteral("Temp=20;Ex12=1.5"), 256).isNull());
}

void PicturaRawTest::reapplyRecordsOneMoreState()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawReapply"), 8, 8, QStringLiteral("rgb"),
                                 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20")));

    const int base = view->history_count();
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=-30;Dhze=7")));
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));
    // A non-Basic key (Dehaze) round-trips through the settings text.
    QVERIFY(view->layer_pictura_raw_settings(path).contains(QStringLiteral("Dhze=7")));
}

void PicturaRawTest::ineligibleTargetRecordsNothing()
{
    QVERIFY(window_->newDocument(QStringLiteral("PicturaRawRefuse"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const QString group = view->add_group_in(QString());
    const int base = view->history_count();
    QVERIFY(!view->apply_pictura_raw_filter(group, QStringLiteral("Temp=10")));
    QCOMPARE(view->history_count(), base);
    QVERIFY(view->pictura_raw_preview(group, QStringLiteral("Temp=10"), 256).isNull());
}

QTEST_MAIN(PicturaRawTest)
#include "tst_pictura_raw.moc"
