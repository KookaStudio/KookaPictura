#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QTabWidget>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "reduce_noise_dialog.h"

#include "qt_test_support.h"

namespace {

const QStringList kReduceNoisePath = {QStringLiteral("Filter"), QStringLiteral("Noise"),
                                      QStringLiteral("Reduce Noise")};

const pictura::FilterCommandSpec& reduceNoiseSpec()
{
    return *pictura::filterCommandForPath(kReduceNoisePath);
}

pictura::ReduceNoiseDialog* openReduceNoiseDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::ReduceNoiseDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

// CS6's defaults, in mapping order: Strength 6, everything else off, and each
// channel's pair equal to the composite.
QList<double> defaults()
{
    return {6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 6.0, 0.0, 6.0, 0.0, 6.0, 0.0};
}

// A noisy picture so the default denoise actually changes pixels.
QImage noisyPicture()
{
    QImage image(120, 80, QImage::Format_RGB32);
    quint32 state = 0x1234u;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            state = state * 1664525u + 1013904223u;
            const int jitter = static_cast<int>((state >> 24) % 41) - 20;
            const int v = qBound(0, 128 + jitter, 255);
            image.setPixel(x, y, qRgb(v, v, v));
        }
    }
    return image;
}

} // namespace

class ReduceNoiseDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layoutFollowsCs6();
    void advancedFlagTracksTheTab();
    void setValuesRoundTripsPerChannel();
    void rejectRestoresPixels();
    void menuOpensItAndCommits();
    void lastFilterSettingsReopensItPrefilled();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ReduceNoiseDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    const QString path = dir_.filePath(QStringLiteral("noise.png"));
    QVERIFY(noisyPicture().save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kReduceNoisePath) != nullptr);
}

void ReduceNoiseDialogTest::layoutFollowsCs6()
{
    pictura::ReduceNoiseDialog dialog(window_->activeView(), reduceNoiseSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Reduce Noise"));
    QCOMPARE(dialog.values(), defaults());

    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("reduceNoiseTabs"));
    QVERIFY(tabs != nullptr);
    QCOMPARE(tabs->count(), 2);
    QCOMPARE(tabs->tabText(0), QStringLiteral("Basic"));
    QCOMPARE(tabs->tabText(1), QStringLiteral("Advanced"));
    QCOMPARE(tabs->currentIndex(), 0);

    auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("reduceNoiseChannel"));
    QVERIFY(channel != nullptr);
    QCOMPARE(channel->count(), 3);
    QCOMPARE(channel->currentText(), QStringLiteral("Red"));

    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"));
    QVERIFY(preview != nullptr);
    QVERIFY(preview->isChecked());
    dialog.reject();
}

void ReduceNoiseDialogTest::advancedFlagTracksTheTab()
{
    pictura::ReduceNoiseDialog dialog(window_->activeView(), reduceNoiseSpec());
    QCOMPARE(dialog.values().at(5), 0.0);
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("reduceNoiseTabs"));
    QVERIFY(tabs != nullptr);
    tabs->setCurrentIndex(1);
    QCOMPARE(dialog.values().at(5), 1.0);
    tabs->setCurrentIndex(0);
    QCOMPARE(dialog.values().at(5), 0.0);
    dialog.reject();
}

void ReduceNoiseDialogTest::setValuesRoundTripsPerChannel()
{
    pictura::ReduceNoiseDialog dialog(window_->activeView(), reduceNoiseSpec());
    const QList<double> v = {8.0, 30.0, 40.0, 20.0, 1.0, 1.0, 2.0, 10.0, 4.0, 50.0, 9.0, 90.0};
    dialog.setValues(v);
    QCOMPARE(dialog.values(), v);
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("reduceNoiseTabs"));
    QVERIFY(tabs != nullptr);
    QCOMPARE(tabs->currentIndex(), 1);
    dialog.reject();
}

void ReduceNoiseDialogTest::rejectRestoresPixels()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::ReduceNoiseDialog dialog(view, reduceNoiseSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY(view->image() != before); // the opening preview has run
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void ReduceNoiseDialogTest::menuOpensItAndCommits()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (auto* dialog = openReduceNoiseDialog()) {
            opened = true;
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kReduceNoisePath)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("reduce-noise"));
    QCOMPARE(filter_last_params(*view), defaults());
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Reduce Noise"));
}

void ReduceNoiseDialogTest::lastFilterSettingsReopensItPrefilled()
{
    pictura::PictureView* view = window_->activeView();
    const QList<double> committed = {7.0, 20.0, 30.0, 10.0, 1.0, 1.0, 2.0, 5.0, 4.0, 15.0, 9.0, 60.0};
    QVERIFY(apply_filter_params(*view, QStringLiteral("reduce-noise"), committed));
    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = openReduceNoiseDialog()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, committed);
}

QTEST_MAIN(ReduceNoiseDialogTest)
#include "tst_reduce_noise_dialog.moc"
