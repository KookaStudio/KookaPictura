// Convert to Profile preview and commit through the colour-management bridge.

#include <QtTest/QtTest>

#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

class ProfileDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void previewShowsConversion();
    void previewRepreviews();
    void previewCancelRestores();
    void commitAddsOneState();

private:
    pictura::PictureView* newFilled(const QColor& color);
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ProfileDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

pictura::PictureView* ProfileDialogTest::newFilled(const QColor& color)
{
    if (!window_->newDocument(QStringLiteral("Profile"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, color.rgba(), -1, QStringLiteral("normal"), 100, false);
    view->deselect();
    return view;
}

void ProfileDialogTest::previewShowsConversion()
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220));
    QVERIFY(view);
    const unsigned before = view->sample_argb(2, 2);
    const int history = view->history_count();
    QVERIFY(pictura::preview_convert_profile(*view, 1, 1, true, false, false));
    QVERIFY(view->sample_argb(2, 2) != before);
    QCOMPARE(view->history_count(), history);
    closeActive();
}

void ProfileDialogTest::previewRepreviews()
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220));
    QVERIFY(view);
    QVERIFY(pictura::preview_convert_profile(*view, 1, 1, true, false, false));
    QVERIFY(pictura::preview_convert_profile(*view, 2, 1, true, false, false));
    const unsigned preview = view->sample_argb(2, 2);
    QVERIFY(pictura::cancel_convert_preview(*view));
    QVERIFY(pictura::convert_profile_with_options(*view, 2, 1, true, false, false));
    QCOMPARE(view->sample_argb(2, 2), preview);
    closeActive();
}

void ProfileDialogTest::previewCancelRestores()
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220));
    QVERIFY(view);
    const unsigned before = view->sample_argb(2, 2);
    const int history = view->history_count();
    QVERIFY(pictura::preview_convert_profile(*view, 1, 1, true, false, false));
    QVERIFY(pictura::cancel_convert_preview(*view));
    QCOMPARE(view->sample_argb(2, 2), before);
    QCOMPARE(view->history_count(), history);
    closeActive();
}

void ProfileDialogTest::commitAddsOneState()
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220));
    QVERIFY(view);
    const int history = view->history_count();
    QVERIFY(pictura::preview_convert_profile(*view, 1, 1, true, false, false));
    const unsigned preview = view->sample_argb(2, 2);
    QVERIFY(pictura::convert_profile_with_options(*view, 1, 1, true, false, false));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->sample_argb(2, 2), preview);
    closeActive();
}

QTEST_MAIN(ProfileDialogTest)
#include "tst_profile_dialog.moc"
