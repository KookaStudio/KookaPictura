// View > Proof Setup and Proof Colors (`color/proof-setup`): the menu state,
// the canvas proof display, persistence, and the Custom dialog.

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtTest/QtTest>
#include <QtWidgets/QApplication>

#include <memory>

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "api/proofing_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "proof_setup_dialog.h"

#include "qt_test_support.h"

namespace {

QString proofFixture()
{
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/metadata.psd");
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
        const QString candidate =
            dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/metadata.psd"));
        if (QFileInfo::exists(candidate)) {
            fixture = candidate;
        } else if (!dir.cdUp()) {
            break;
        }
    }
    return fixture;
}

} // namespace

class ProofSetupTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void defaultsAreWorkingCmyk();
    void setupSurvivesRestart();
    void menuSelectsSetup();
    void proofColorsToggles();
    void titleNamesProofSetup();
    void canvasShowsProof();
    void proofingIsViewOnly();
    void proofSkipsMaskViews();
    void customDialogAppliesAndCancelKeeps();

private:
    pictura::PictureView* whiteDocument();
    QAction* action(const char* id);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ProofSetupTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ProofSetupTest::init()
{
    if (!window_) {
        window_ = pictura::test::makeMainWindow();
    }
    if (!window_ || !window_->newDocument(QStringLiteral("Reset"), 1, 1,
                                          QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return;
    }
    if (QAction* cmyk = action(pictura::command_ids::ViewProofSetupWorkingCmyk)) {
        cmyk->trigger();
    }
    if (QAction* colors = action(pictura::command_ids::ViewProofColors);
        colors && colors->isChecked()) {
        colors->trigger();
    }
    window_->closeActiveDocument(false);
}

void ProofSetupTest::cleanup()
{
    if (window_ && window_->activeView() && window_->activeView()->has_document()) {
        window_->closeActiveDocument(false);
    }
}

pictura::PictureView* ProofSetupTest::whiteDocument()
{
    if (!window_->newDocument(QStringLiteral("Proof"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

QAction* ProofSetupTest::action(const char* id)
{
    return window_->registry()->action(QString::fromLatin1(id));
}

void ProofSetupTest::defaultsAreWorkingCmyk()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);
    QCOMPARE(pictura::proof_device(*view), 0);
    QCOMPARE(pictura::proof_intent(*view), 1);
    QVERIFY(pictura::proof_bpc(*view));
    QVERIFY(!pictura::proof_simulate_paper(*view));
    QVERIFY(!pictura::proof_simulate_black(*view));
    QVERIFY(!pictura::proof_colors_active(*view));
    QCOMPARE(pictura::proof_setup_name(*view), QStringLiteral("Working CMYK"));
    window_->closeActiveDocument(false);
}

void ProofSetupTest::setupSurvivesRestart()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);
    QVERIFY(action(pictura::command_ids::ViewProofSetupWorkingGray));
    action(pictura::command_ids::ViewProofSetupWorkingGray)->trigger();
    action(pictura::command_ids::ViewProofColors)->trigger();
    QCOMPARE(pictura::proof_device(*view), 1);
    QVERIFY(pictura::proof_colors_active(*view));
    window_->saveSessionNow();

    window_.reset();
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    view = whiteDocument();
    QVERIFY(view);
    QCOMPARE(pictura::proof_device(*view), 1);
    QVERIFY(pictura::proof_colors_active(*view));
    window_->closeActiveDocument(false);
}

void ProofSetupTest::menuSelectsSetup()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);
    QAction* cmyk = action(pictura::command_ids::ViewProofSetupWorkingCmyk);
    QAction* gray = action(pictura::command_ids::ViewProofSetupWorkingGray);
    QVERIFY(cmyk && gray);
    QVERIFY(cmyk->isChecked());
    gray->trigger();
    QCOMPARE(pictura::proof_device(*view), 1);
    QVERIFY(gray->isChecked());
    QVERIFY(!cmyk->isChecked());
    window_->closeActiveDocument(false);
}

void ProofSetupTest::proofColorsToggles()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);
    QAction* colors = action(pictura::command_ids::ViewProofColors);
    QVERIFY(colors);
    colors->trigger();
    QVERIFY(pictura::proof_colors_active(*view));
    QVERIFY(colors->isChecked());
    colors->trigger();
    QVERIFY(!pictura::proof_colors_active(*view));
    QVERIFY(!colors->isChecked());
    window_->closeActiveDocument(false);
}

void ProofSetupTest::titleNamesProofSetup()
{
    QVERIFY(whiteDocument());
    action(pictura::command_ids::ViewProofSetupWorkingGray)->trigger();
    action(pictura::command_ids::ViewProofColors)->trigger();
    QVERIFY2(window_->windowTitle().contains(QStringLiteral("Working Gray")),
             qPrintable(window_->windowTitle()));
    window_->closeActiveDocument(false);
}

void ProofSetupTest::canvasShowsProof()
{
    const QString fixture = proofFixture();
    QVERIFY2(!fixture.isEmpty(), "metadata.psd fixture not found");
    QVERIFY(window_->openDocumentAtPath(fixture, false));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    pictura::set_proof_setup(*view, 0, 1, true, true, false);
    const QImage before = view->canvas_display_image(0, 0, 0, 1, 1);
    action(pictura::command_ids::ViewProofColors)->trigger();
    const QImage after = view->canvas_display_image(0, 0, 0, 1, 1);
    QVERIFY2(before.pixelColor(0, 0) != after.pixelColor(0, 0),
             "the canvas must show the soft-proof preview");
    window_->closeActiveDocument(false);
}

void ProofSetupTest::proofingIsViewOnly()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);
    pictura::set_proof_setup(*view, 0, 1, true, true, false);
    const int depth = view->history_depth();
    const bool dirty = pictura::document_api::isDirty(*view);
    const QImage before = view->canvas_display_image(0, 0, 0, 1, 1);
    action(pictura::command_ids::ViewProofColors)->trigger();
    action(pictura::command_ids::ViewProofColors)->trigger();
    QCOMPARE(view->history_depth(), depth);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
    QCOMPARE(before.pixelColor(0, 0), view->canvas_display_image(0, 0, 0, 1, 1).pixelColor(0, 0));
    window_->closeActiveDocument(false);
}

void ProofSetupTest::proofSkipsMaskViews()
{
    const QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/masked.psd");
    QVERIFY2(!fixture.isEmpty(), "masked.psd fixture not found");
    QVERIFY(window_->openDocumentAtPath(fixture, false));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);

    QVERIFY(pictura::mask_solo_set(*view, QStringLiteral("0")));
    pictura::set_proof_setup(*view, 0, 1, true, true, false);
    const QImage soloBefore = view->canvas_display_image(0, 0, 0, 1, 1);
    action(pictura::command_ids::ViewProofColors)->trigger();
    const QImage soloAfter = view->canvas_display_image(0, 0, 0, 1, 1);
    QCOMPARE(soloBefore.pixelColor(0, 0), soloAfter.pixelColor(0, 0));
    QVERIFY(pictura::mask_solo_set(*view, QString()));

    view->select_all();
    QVERIFY(pictura::quick_mask_set(*view, true));
    const QImage quickBefore = view->canvas_display_image(0, 0, 0, 1, 1);
    action(pictura::command_ids::ViewProofColors)->trigger();
    const QImage quickAfter = view->canvas_display_image(0, 0, 0, 1, 1);
    QCOMPARE(quickBefore.pixelColor(0, 0), quickAfter.pixelColor(0, 0));
    window_->closeActiveDocument(false);
}

void ProofSetupTest::customDialogAppliesAndCancelKeeps()
{
    pictura::PictureView* view = whiteDocument();
    QVERIFY(view);

    QTimer::singleShot(0, []() {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<pictura::ProofSetupDialog*>(widget)) {
                dialog->setDeviceForTest(3);
                dialog->setSimulateBlackForTest(true);
                dialog->accept();
                return;
            }
        }
    });
    action(pictura::command_ids::ViewProofSetupCustom)->trigger();
    QCOMPARE(pictura::proof_device(*view), 3);
    QVERIFY(pictura::proof_simulate_black(*view));

    QTimer::singleShot(0, []() {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<pictura::ProofSetupDialog*>(widget)) {
                dialog->setDeviceForTest(4);
                dialog->reject();
                return;
            }
        }
    });
    action(pictura::command_ids::ViewProofSetupCustom)->trigger();
    QCOMPARE(pictura::proof_device(*view), 3);
    QVERIFY(pictura::proof_simulate_black(*view));
    window_->closeActiveDocument(false);
}

QTEST_MAIN(ProofSetupTest)
#include "tst_proof_setup.moc"
