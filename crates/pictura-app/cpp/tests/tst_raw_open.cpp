// Camera raw opens through File > Open (#400, `document/raw-open`): signature
// routing, the Pictura Raw dialog before the document, the non-interactive
// develop, the absolute white balance, and the unsupported-camera refusal. The
// fixture DNG is written by `raw_fixture.h`. The suite shares one frame, never
// shown; cleanup() closes its documents and clears the dialog driver and the
// status message.

#include <QtTest/QtTest>

#include "acr/acr_controls.h"
#include "acr/acr_panels.h"
#include "acr/acr_settings.h"
#include "dialogs.h"
#include "frame.h"
#include "pictura_raw_dialog.h"

#include "api/document_bridge.h"

#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QStatusBar>

#include "qt_test_support.h"
#include "raw_fixture.h"

using namespace pictura;

namespace {

QString writeFile(const QTemporaryDir& dir, const QString& name, const QByteArray& bytes)
{
    const QString path = dir.filePath(name);
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size()) {
        return {};
    }
    return path;
}

// What the dialog driver saw.
struct Seen {
    int rawDialogs = 0;
    int otherDialogs = 0;
    int documentsAtDialog = -1;
};

} // namespace

class RawOpenTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void cleanupTestCase();

    void openFiltersListCameraRaw();
    void dialogOkOpensSixteenBit();
    void dialogCancelOpensNothing();
    void openWithoutDialog();
    void unsupportedColourFailsClearly();
    void rawDialogShowsAbsoluteSliders();
    void reciprocalSliderSpreadsDaylight();

private:
    std::function<bool(QDialog&)> driver(Seen* seen, bool accept);

    test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void RawOpenTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void RawOpenTest::init()
{
    QVERIFY(window_ != nullptr);
    QCOMPARE(window_->documentCount(), 0);
}

void RawOpenTest::cleanup()
{
    if (!window_) {
        return;
    }
    window_->cancelAllOpens();
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
    window_->setImportDialogDriverForTest({});
    window_->statusBar()->clearMessage();
}

void RawOpenTest::cleanupTestCase()
{
    window_.reset();
}

std::function<bool(QDialog&)> RawOpenTest::driver(Seen* seen, bool accept)
{
    PicturaMainWindow* window = window_.get();
    return [=](QDialog& dialog) {
        if (!qobject_cast<PicturaRawDialog*>(&dialog)) {
            ++seen->otherDialogs;
            return true;
        }
        ++seen->rawDialogs;
        seen->documentsAtDialog = 0;
        for (int i = 0; i < window->documentCount(); ++i) {
            seen->documentsAtDialog += window->isLoadingDocument(i) ? 0 : 1;
        }
        return accept;
    };
}

// raw_open_filter
void RawOpenTest::openFiltersListCameraRaw()
{
    const QStringList& filters = openFileFilters();
    QVERIFY(filters.contains(QStringLiteral("Camera Raw (*.DNG *.CR2)")));
    const QString all = filters.first();
    QVERIFY(all.contains(QLatin1String("*.DNG")));
    QVERIFY(all.contains(QLatin1String("*.CR2")));
}

// raw_open_dialog_ok
void RawOpenTest::dialogOkOpensSixteenBit()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    QVERIFY(!path.isEmpty());
    Seen seen;
    window_->setImportDialogDriverForTest(driver(&seen, true));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(seen.rawDialogs, 1);
    QCOMPARE(seen.documentsAtDialog, 0);
    QCOMPARE(window_->documentCount(), 1);
    PictureView* view = window_->viewAt(0);
    QVERIFY(view);
    QCOMPARE(view->document_depth_bits(), 16);
    QCOMPARE(view->document_mode(), QStringLiteral("rgb"));
    QCOMPARE(view->document_width(), 16);
    QCOMPARE(view->document_height(), 12);
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), 1);
}

// raw_open_dialog_cancel
void RawOpenTest::dialogCancelOpensNothing()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    Seen seen;
    window_->setImportDialogDriverForTest(driver(&seen, false));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(seen.rawDialogs, 1);
    QCOMPARE(window_->documentCount(), 0);
}

// raw_open_noninteractive
void RawOpenTest::openWithoutDialog()
{
    QTemporaryDir dir;
    // The extension does not route a raw file; its signature does.
    const QString path = writeFile(dir, QStringLiteral("shot.tif"), test::rawDng(16, 12, true));
    Seen seen;
    window_->setImportDialogDriverForTest(driver(&seen, true));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, false),
             PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(seen.rawDialogs, 0);
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->viewAt(0)->document_depth_bits(), 16);
    QCOMPARE(window_->viewAt(0)->document_width(), 16);
}

// raw_open_unsupported
void RawOpenTest::unsupportedColourFailsClearly()
{
    QTemporaryDir dir;
    const QString path =
        writeFile(dir, QStringLiteral("nocolour.dng"), test::rawDng(16, 12, false));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, false),
             PicturaMainWindow::OpenOutcome::Failed);
    QTRY_COMPARE(window_->documentCount(), 0);
    const QString message = window_->statusBar()->currentMessage();
    QVERIFY2(message.contains(QStringLiteral("nocolour.dng")), qPrintable(message));
    QVERIFY2(message.contains(QStringLiteral("camera colour")), qPrintable(message));
}

// raw_open_absolute_sliders
void RawOpenTest::rawDialogShowsAbsoluteSliders()
{
    AcrSettings seed = AcrSettings::decode(QString());
    seed.setInt(acr::Temp, 5500);
    seed.setInt(acr::Tint, 10);
    PicturaRawDialog raw(nullptr, AcrAsShot{5500, 10}, seed.encode(), nullptr);
    auto* temperature = raw.findChild<AcrSlider*>(QString::fromLatin1(acr::Temp));
    auto* tint = raw.findChild<AcrSlider*>(QString::fromLatin1(acr::Tint));
    QVERIFY(temperature && tint);
    QCOMPARE(temperature->minimum(), 2000.0);
    QCOMPARE(temperature->maximum(), 50000.0);
    QCOMPARE(temperature->value(), 5500.0);
    QCOMPARE(tint->minimum(), -150.0);
    QCOMPARE(tint->maximum(), 150.0);

    PicturaRawDialog layer(nullptr, QString(), QString(), nullptr);
    auto* relative = layer.findChild<AcrSlider*>(QString::fromLatin1(acr::Temp));
    QVERIFY(relative);
    QCOMPARE(relative->minimum(), -100.0);
    QCOMPARE(relative->maximum(), 100.0);
}

// raw_open_reciprocal_slider
void RawOpenTest::reciprocalSliderSpreadsDaylight()
{
    AcrSlider slider(QStringLiteral("Temperature"), 2000.0, 50000.0, 6500.0, 0);
    slider.setReciprocalScale(true);
    slider.resize(212, 40);
    QTest::mouseClick(&slider, Qt::LeftButton, Qt::NoModifier,
                      QPoint(slider.width() / 2, slider.height() - 8));
    QVERIFY2(slider.value() >= 3500.0 && slider.value() <= 4500.0,
             qPrintable(QString::number(slider.value())));
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<RawOpenTest>(argc, argv);
}
#include "tst_raw_open.moc"
