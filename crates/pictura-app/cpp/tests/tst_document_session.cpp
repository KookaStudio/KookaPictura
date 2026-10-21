// The document-session boundary: cancellation reports and the replacement
// entry-point signal policy.

#include <QtTest/QtTest>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"

class DocumentSessionTest : public QObject {
    Q_OBJECT

private slots:
    void regionBlitDoesNotEmitChanged();
    void rebuildEmitsChanged();
    void replaceEntryPointsCancelTheSession();
    void replacementSignalsMatchTheEntryPoint();
};

void DocumentSessionTest::regionBlitDoesNotEmitChanged()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(view.begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false));
    QVERIFY(view.paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view.cancel_active_session());
    QVERIFY2(blits.count() >= 1, "cancel blits the stroke region");
    QCOMPARE(changes.count(), 0);
}

void DocumentSessionTest::rebuildEmitsChanged()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(pictura::preview_indexed(view, 1, 256, 0, 0));
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(view.cancel_active_session());
    QCOMPARE(changes.count(), 1);
}

void DocumentSessionTest::replaceEntryPointsCancelTheSession()
{
    pictura::PictureView view;
    QSignalSpy cancelled(&view, &pictura::PictureView::sessionCancelled);
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCOMPARE(cancelled.count(), 1);
}

void DocumentSessionTest::replacementSignalsMatchTheEntryPoint()
{
    pictura::PictureView view;
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCOMPARE(changes.count(), 0);
    QCOMPARE(blits.count(), 0);
}

QTEST_MAIN(DocumentSessionTest)
#include "tst_document_session.moc"
