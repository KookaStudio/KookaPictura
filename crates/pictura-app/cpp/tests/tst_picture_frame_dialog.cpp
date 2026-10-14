#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QTabWidget>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "picture_frame_dialog.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kPictureFramePath = {QStringLiteral("Filter"), QStringLiteral("Render"),
                                       QStringLiteral("Picture Frame")};

const pictura::FilterCommandSpec& pictureFrameSpec()
{
    return *pictura::filterCommandForPath(kPictureFramePath);
}

pictura::PictureFrameDialog* openPictureFrameDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::PictureFrameDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

// CS6's defaults, in mapping order.
QList<double> defaults()
{
    return {33.0, 60.0, 84.0, 34.0, 14.0, 20.0, 1.0, 2.0, 0.0, 88.0, 218.0,
            20.0, 4.0, 104.0, 170.0, 60.0, 20.0, 15.0, 29.0, 10.0, 0.0, 0.0};
}

int ink(const QImage& image)
{
    int count = 0;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            const QColor c = image.pixelColor(x, y);
            if (c.red() < 250 || c.green() < 250 || c.blue() < 250) {
                ++count;
            }
        }
    }
    return count;
}

} // namespace

class PictureFrameDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layoutFollowsCs6();
    void pickerAndControlsDriveTheFrame();
    void rejectRestoresPixels();
    void menuOpensItAndCommitsTheFrame();
    void lastFilterSettingsReopensItPrefilled();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PictureFrameDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QImage white(120, 80, QImage::Format_RGB32);
    white.fill(QColor(255, 255, 255));
    const QString path = dir_.filePath(QStringLiteral("white.png"));
    QVERIFY(white.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kPictureFramePath) != nullptr);
}

void PictureFrameDialogTest::layoutFollowsCs6()
{
    pictura::PictureFrameDialog dialog(window_->activeView(), pictureFrameSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Picture Frame"));
    QCOMPARE(dialog.values(), defaults());

    auto* picker = dialog.findChild<QListWidget*>(QStringLiteral("pictureFramePicker"));
    QVERIFY(picker != nullptr);
    QCOMPARE(picker->count(), 47);
    QCOMPARE(picker->currentRow(), 33);
    QCOMPARE(picker->item(0)->text(), QStringLiteral("Happy Vine"));
    QCOMPARE(picker->item(46)->text(), QStringLiteral("Dual Rounded Art Frame 2"));

    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("pictureFrameTabs"));
    QVERIFY(tabs != nullptr);
    QCOMPARE(tabs->count(), 2);
    QCOMPARE(tabs->tabText(0), QStringLiteral("Basic"));
    QCOMPARE(tabs->tabText(1), QStringLiteral("Advanced"));

    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"));
    QVERIFY(preview != nullptr);
    QVERIFY(preview->isChecked());

    auto* pad = dialog.findChild<QWidget*>(QStringLiteral("pictureFramePad"));
    QVERIFY(pad != nullptr);
    QVERIFY(!dialog.padImage().isNull());
    QVERIFY2(ink(dialog.padImage()) > 100, "the frame drew no ink on the pad");
    dialog.reject();
}

void PictureFrameDialogTest::pickerAndControlsDriveTheFrame()
{
    pictura::PictureFrameDialog dialog(window_->activeView(), pictureFrameSpec());
    auto* picker = dialog.findChild<QListWidget*>(QStringLiteral("pictureFramePicker"));
    QVERIFY(picker != nullptr);

    picker->setCurrentRow(13); // Chainmail
    QCOMPARE(dialog.values().first(), 13.0);
    const QImage chainmail = dialog.padImage();

    QList<double> noFlowers = defaults();
    noFlowers[7] = 0.0; // flower = None
    noFlowers[0] = 34.0; // Pinwheel (draws flowers)
    dialog.setValues(noFlowers);
    const QImage without = dialog.padImage();
    QList<double> withFlowers = noFlowers;
    withFlowers[7] = 2.0;
    dialog.setValues(withFlowers);
    QVERIFY(dialog.padImage() != without);
    QVERIFY(chainmail != without);
    dialog.reject();
}

void PictureFrameDialogTest::rejectRestoresPixels()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::PictureFrameDialog dialog(view, pictureFrameSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY(view->image() != before); // the opening preview has run
    auto* picker = dialog.findChild<QListWidget*>(QStringLiteral("pictureFramePicker"));
    QVERIFY(picker != nullptr);
    picker->setCurrentRow(0);
    QVERIFY(view->image() != before);
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void PictureFrameDialogTest::menuOpensItAndCommitsTheFrame()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (auto* dialog = openPictureFrameDialog()) {
            opened = true;
            auto* picker = dialog->findChild<QListWidget*>(QStringLiteral("pictureFramePicker"));
            if (picker) {
                picker->setCurrentRow(13); // Chainmail
            }
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kPictureFramePath)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("picture-frame"));
    QList<double> expected = defaults();
    expected[0] = 13.0;
    QCOMPARE(filter_last_params(*view), expected);
    QVERIFY(view->image() != QImage());
}

void PictureFrameDialogTest::lastFilterSettingsReopensItPrefilled()
{
    pictura::PictureView* view = window_->activeView();
    QList<double> committed = defaults();
    committed[0] = 20.0;  // Snakes
    committed[4] = 40.0;  // margin
    committed[13] = 8.0;  // leaf colour red
    QVERIFY(apply_filter_params(*view, QStringLiteral("picture-frame"), committed));
    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = openPictureFrameDialog()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, committed);
}

QTEST_MAIN(PictureFrameDialogTest)
#include "tst_picture_frame_dialog.moc"
