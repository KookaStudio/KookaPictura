#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "liquify_canvas.h"
#include "liquify_dialog.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/liquify.cxxqt.h"

#include "qt_test_support.h"

using pictura::LiquifyDialog;

namespace {

const QStringList kLiquifyPath = {QStringLiteral("Filter"), QStringLiteral("Liquify…")};

LiquifyDialog* openLiquifyDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<LiquifyDialog*>(widget); dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

QToolButton* toolButton(const LiquifyDialog& dialog, int tool)
{
    return dialog.findChild<QToolButton*>(QStringLiteral("liquifyTool%1").arg(tool));
}

// A drag across the stripes, in layer pixels.
const QList<QPointF> kDrag = {QPointF(50, 60), QPointF(70, 60), QPointF(90, 62)};

} // namespace

class LiquifyDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuRowIsLiveWithItsShortcut();
    void basicModeShowsCs6sShortColumn();
    void advancedModeAddsTheRest();
    void opensAtActualPixels();
    void zoomMenuOffersCs6Levels();
    void aMagnifiedViewIsSmooth();
    void toolShortcutsFollowTheMode();
    void aDragDistortsThePreviewNotTheDocument();
    void restoreAllAndReconstructUndoTheDistortion();
    void aFrozenLayerTakesNoDistortion();
    void cancelLeavesTheDocumentAndHistory();
    void anUndistortedOkRecordsNothing();
    void okCommitsOneFilterState();
    void loadLastMeshReappliesTheCommittedMesh();
    void lastFilterSettingsReopensWithTheMesh();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LiquifyDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // Vertical stripes: any sideways push shows.
    QImage stripes(160, 120, QImage::Format_RGB32);
    for (int y = 0; y < stripes.height(); ++y) {
        for (int x = 0; x < stripes.width(); ++x) {
            stripes.setPixelColor(x, y, (x / 6) % 2 ? QColor(230, 40, 60) : QColor(20, 90, 200));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("stripes.png"));
    QVERIFY(stripes.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
}

void LiquifyDialogTest::menuRowIsLiveWithItsShortcut()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kLiquifyPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("liquify"));
    QVERIFY(pictura::filter_kind_supported(spec->kind));
    QCOMPARE(pictura::filter_param_arity(spec->kind), 1);
    const QString id = pictura::commandIdForPath(kLiquifyPath);
    QAction* action = window_->registry()->action(id);
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Shift+Ctrl+X")));
}

void LiquifyDialogTest::basicModeShowsCs6sShortColumn()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    QVERIFY(dialog.ready());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QVERIFY2(dialog.windowTitle().startsWith(QStringLiteral("Liquify (stripes.png @ ")),
             qPrintable(dialog.windowTitle()));
    QVERIFY(!dialog.advanced());
    for (int tool : {LiquifyDialog::ForwardWarp, LiquifyDialog::Reconstruct, LiquifyDialog::Pucker,
                     LiquifyDialog::Bloat, LiquifyDialog::PushLeft, LiquifyDialog::Hand,
                     LiquifyDialog::Zoom}) {
        QVERIFY2(toolButton(dialog, tool)->isVisible(), qPrintable(QString::number(tool)));
    }
    for (int tool : {LiquifyDialog::Twirl, LiquifyDialog::Mirror, LiquifyDialog::Turbulence,
                     LiquifyDialog::Freeze, LiquifyDialog::Thaw}) {
        QVERIFY2(!toolButton(dialog, tool)->isVisible(), qPrintable(QString::number(tool)));
    }
    QCOMPARE(toolButton(dialog, LiquifyDialog::Pucker)->toolTip(), QStringLiteral("Pucker Tool (S)"));
    QCOMPARE(dialog.tool(), int(LiquifyDialog::ForwardWarp));
    auto* size = dialog.findChild<QSpinBox*>(QStringLiteral("liquifySize"));
    auto* pressure = dialog.findChild<QSpinBox*>(QStringLiteral("liquifyPressure"));
    QVERIFY(size && pressure);
    QCOMPARE(size->value(), 100);
    QCOMPARE(size->maximum(), 15000);
    QCOMPARE(pressure->value(), 100);
    QVERIFY(size->isVisible() && pressure->isVisible());
    QVERIFY(!dialog.findChild<QSpinBox*>(QStringLiteral("liquifyDensity"))->isVisible());
    QCOMPARE(dialog.findChild<QGroupBox*>(QStringLiteral("liquifyToolOptions"))->title(),
             QStringLiteral("Brush Tool Options"));
    QVERIFY(dialog.findChild<QPushButton*>(QStringLiteral("liquifyRestoreAll"))->isVisible());
    QVERIFY(!dialog.findChild<QPushButton*>(QStringLiteral("liquifyReconstruct"))->isVisible());
    // No earlier OK has kept a mesh in this run yet.
    QVERIFY(!dialog.findChild<QPushButton*>(QStringLiteral("liquifyLoadLastMesh"))->isEnabled());
    dialog.reject();
}

void LiquifyDialogTest::advancedModeAddsTheRest()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    dialog.findChild<QCheckBox*>(QStringLiteral("liquifyAdvanced"))->setChecked(true);
    QVERIFY(dialog.advanced());
    for (int tool = LiquifyDialog::ForwardWarp; tool <= LiquifyDialog::Zoom; ++tool) {
        QVERIFY2(toolButton(dialog, tool)->isVisible(), qPrintable(QString::number(tool)));
    }
    for (const char* name : {"liquifyDensity", "liquifyRate", "liquifyJitter"}) {
        QVERIFY2(dialog.findChild<QSpinBox*>(QString::fromLatin1(name))->isVisible(), name);
    }
    QCOMPARE(dialog.findChild<QSpinBox*>(QStringLiteral("liquifyDensity"))->value(), 50);
    QCOMPARE(dialog.findChild<QSpinBox*>(QStringLiteral("liquifyRate"))->value(), 80);
    QVERIFY(dialog.findChild<QPushButton*>(QStringLiteral("liquifyReconstruct"))->isVisible());
    QVERIFY(dialog.findChild<QPushButton*>(QStringLiteral("liquifyMaskAll"))->isVisible());
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("liquifyShowMask"))->isVisible());

    // Back to Basic with an Advanced-only tool picks Forward Warp.
    dialog.selectTool(LiquifyDialog::Mirror);
    dialog.setAdvanced(false);
    QCOMPARE(dialog.tool(), int(LiquifyDialog::ForwardWarp));
    dialog.reject();
}

void LiquifyDialogTest::opensAtActualPixels()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QCOMPARE(dialog.canvas()->zoom(), 1.0);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Liquify (stripes.png @ 100%)"));
    dialog.reject();
}

void LiquifyDialogTest::zoomMenuOffersCs6Levels()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QObject owner;
    const QList<QAction*> actions = dialog.canvas()->zoomMenuActions(&owner);
    QStringList labels;
    for (QAction* action : actions) {
        labels.append(action->isSeparator() ? QStringLiteral("-") : action->text());
    }
    QCOMPARE(labels, QStringList({"6%", "12%", "25%", "33%", "50%", "66%", "-", "100%", "-",
                                  "200%", "300%", "400%", "600%", "800%", "1000%", "1200%",
                                  "1600%", "-", "Actual Pixels", "Fit in View", "Fit on Screen"}));
    auto trigger = [&](const QString& text) {
        for (QAction* action : actions) {
            if (action->text() == text) {
                action->trigger();
            }
        }
    };
    trigger(QStringLiteral("400%"));
    QCOMPARE(dialog.canvas()->zoom(), 4.0);
    QVERIFY(dialog.windowTitle().endsWith(QStringLiteral("@ 400%)")));
    trigger(QStringLiteral("Actual Pixels"));
    QCOMPARE(dialog.canvas()->zoom(), 1.0);
    // The 160 px layer fits a larger frame: Fit on Screen magnifies it, Fit in
    // View stops at 100%.
    trigger(QStringLiteral("Fit on Screen"));
    QVERIFY(dialog.canvas()->zoom() > 1.0);
    trigger(QStringLiteral("Fit in View"));
    QCOMPARE(dialog.canvas()->zoom(), 1.0);
    dialog.reject();
}

void LiquifyDialogTest::aMagnifiedViewIsSmooth()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    pictura::LiquifyCanvas* canvas = dialog.canvas();
    canvas->setZoom(8.0);
    // Across a stripe edge at 800% the view blends, rather than jumping from
    // one colour to the other in 8 px blocks.
    const QImage view = canvas->previewImage().convertToFormat(QImage::Format_RGB32);
    const double dpr = view.devicePixelRatio();
    // The stripe edge nearest the middle of the frame.
    const QPointF middle = canvas->toLayer(QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    const double edgeX = std::round(middle.x() / 6.0) * 6.0;
    const QPointF edge = canvas->toWidget(QPointF(edgeX, middle.y())) * dpr;
    QSet<QRgb> colours;
    for (int dx = -8; dx <= 8; ++dx) {
        colours.insert(view.pixel(static_cast<int>(edge.x()) + dx, static_cast<int>(edge.y())));
    }
    QVERIFY2(colours.size() >= 6, qPrintable(QString::number(colours.size())));
    dialog.reject();
}

void LiquifyDialogTest::toolShortcutsFollowTheMode()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    dialog.canvas()->setFocus();
    QTest::keyClick(&dialog, Qt::Key_S);
    QCOMPARE(dialog.tool(), int(LiquifyDialog::Pucker));
    QTest::keyClick(&dialog, Qt::Key_C); // Twirl is Advanced-only
    QCOMPARE(dialog.tool(), int(LiquifyDialog::Pucker));
    dialog.setAdvanced(true);
    QTest::keyClick(&dialog, Qt::Key_C);
    QCOMPARE(dialog.tool(), int(LiquifyDialog::Twirl));
    auto* size = dialog.findChild<QSpinBox*>(QStringLiteral("liquifySize"));
    QTest::keyClick(&dialog, Qt::Key_BracketRight);
    QCOMPARE(size->value(), 110);
    QTest::keyClick(&dialog, Qt::Key_BracketLeft);
    QCOMPARE(size->value(), 99);
    dialog.reject();
}

void LiquifyDialogTest::aDragDistortsThePreviewNotTheDocument()
{
    pictura::PictureView* view = window_->activeView();
    const QImage document = view->image();
    LiquifyDialog dialog(view, QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    const QImage before = dialog.canvas()->previewImage();
    QVERIFY(!before.isNull());
    QVERIFY(!pictura::liquify_distorted());
    dialog.canvas()->strokeForTest(kDrag);
    QVERIFY(pictura::liquify_distorted());
    QVERIFY(dialog.canvas()->previewImage() != before);
    QCOMPARE(view->image(), document);
    dialog.reject();
}

void LiquifyDialogTest::restoreAllAndReconstructUndoTheDistortion()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    const QImage before = dialog.canvas()->previewImage();
    dialog.canvas()->strokeForTest(kDrag);
    dialog.findChild<QPushButton*>(QStringLiteral("liquifyRestoreAll"))->click();
    QVERIFY(!pictura::liquify_distorted());
    QCOMPARE(dialog.canvas()->previewImage(), before);

    // Reconstruct at its default 100% restores an unfrozen layer too.
    dialog.setAdvanced(true);
    QCoreApplication::processEvents();
    const QImage advancedBefore = dialog.canvas()->previewImage();
    dialog.canvas()->strokeForTest(kDrag);
    QTimer::singleShot(0, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (widget->windowTitle() == QStringLiteral("Revert Reconstruction") &&
                widget->isVisible()) {
                auto* amount = widget->findChild<QSpinBox*>(QStringLiteral("liquifyReconstructAmount"));
                QCOMPARE(amount->value(), 100);
                qobject_cast<QDialog*>(widget)->accept();
            }
        }
    });
    dialog.findChild<QPushButton*>(QStringLiteral("liquifyReconstruct"))->click();
    QVERIFY(!pictura::liquify_distorted());
    QCOMPARE(dialog.canvas()->previewImage(), advancedBefore);
    dialog.reject();
}

void LiquifyDialogTest::aFrozenLayerTakesNoDistortion()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    dialog.setAdvanced(true);
    const QImage plain = dialog.canvas()->previewImage();
    dialog.findChild<QPushButton*>(QStringLiteral("liquifyMaskAll"))->click();
    dialog.canvas()->strokeForTest(kDrag);
    QVERIFY(!pictura::liquify_distorted());
    // Show Mask tints the frozen layer.
    dialog.findChild<QCheckBox*>(QStringLiteral("liquifyShowMask"))->setChecked(true);
    QVERIFY(dialog.canvas()->previewImage() != plain);
    dialog.findChild<QPushButton*>(QStringLiteral("liquifyMaskInvert"))->click();
    dialog.canvas()->strokeForTest(kDrag);
    QVERIFY(pictura::liquify_distorted());
    dialog.reject();
}

void LiquifyDialogTest::cancelLeavesTheDocumentAndHistory()
{
    pictura::PictureView* view = window_->activeView();
    const QImage document = view->image();
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (LiquifyDialog* dialog = openLiquifyDialog()) {
            opened = true;
            dialog->canvas()->strokeForTest(kDrag);
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLiquifyPath)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);
}

void LiquifyDialogTest::anUndistortedOkRecordsNothing()
{
    pictura::PictureView* view = window_->activeView();
    const int history = view->history_count();
    QTimer::singleShot(0, [] {
        if (LiquifyDialog* dialog = openLiquifyDialog()) {
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLiquifyPath)));
    QCOMPARE(view->history_count(), history);
}

void LiquifyDialogTest::okCommitsOneFilterState()
{
    pictura::PictureView* view = window_->activeView();
    const QImage document = view->image();
    const int history = view->history_count();
    QString title;
    QTimer::singleShot(0, [&title] {
        if (LiquifyDialog* dialog = openLiquifyDialog()) {
            title = dialog->windowTitle();
            dialog->canvas()->strokeForTest(kDrag);
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLiquifyPath)));
    QVERIFY2(title.startsWith(QStringLiteral("Liquify (stripes.png @ ")), qPrintable(title));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("liquify"));
    const QList<double> params = filter_last_params(*view);
    QCOMPARE(params.size(), 1);
    QVERIFY(params.first() > 0.0);
    const QImage after = view->image();
    QVERIFY(after != document);
    // Far from the drag nothing moved.
    QCOMPARE(after.pixelColor(150, 10), document.pixelColor(150, 10));
}

void LiquifyDialogTest::loadLastMeshReappliesTheCommittedMesh()
{
    LiquifyDialog dialog(window_->activeView(), QStringLiteral("stripes.png"));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* load = dialog.findChild<QPushButton*>(QStringLiteral("liquifyLoadLastMesh"));
    QVERIFY(load->isEnabled());
    const QImage before = dialog.canvas()->previewImage();
    QVERIFY(!pictura::liquify_distorted());
    load->click();
    QVERIFY(pictura::liquify_distorted());
    QVERIFY(dialog.canvas()->previewImage() != before);
    dialog.reject();
}

void LiquifyDialogTest::lastFilterSettingsReopensWithTheMesh()
{
    pictura::PictureView* view = window_->activeView();
    const int history = view->history_count();
    bool distorted = false;
    QTimer::singleShot(0, [&distorted] {
        if (LiquifyDialog* dialog = openLiquifyDialog()) {
            distorted = pictura::liquify_distorted();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QVERIFY(distorted);
    QCOMPARE(view->history_count(), history);
}

QTEST_MAIN(LiquifyDialogTest)
#include "tst_liquify_dialog.moc"
