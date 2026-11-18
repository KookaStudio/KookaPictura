// File > Automate > Batch… dialog (#618; ui/batch-dialog): the menu command,
// the option groups and their enablement, the naming example, acceptance
// (validation and the baked-in Open warning), and persisted options.

#include <QtTest/QtTest>

#include "batch_dialog.h"
#include "batch_runner.h"
#include "commands.h"
#include "frame.h"
#include "panels/actions_panel.h"
#include "panels/actions_tree_model.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include <functional>
#include <memory>

#include "qt_test_support.h"

namespace {

QStringList comboItems(const QComboBox* combo)
{
    QStringList items;
    for (int i = 0; i < combo->count(); ++i) {
        items << combo->itemText(i);
    }
    return items;
}

template <typename T>
T* child(const QWidget& parent, const char* name)
{
    return parent.findChild<T*>(QLatin1String(name));
}

} // namespace

class BatchDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void commandOpensTheDialog();
    void dialogShowsEveryGroup();
    void controlsFollowSourceAndDestination();
    void exampleFollowsTheScheme();
    void invalidOptionsKeepTheDialogOpen();
    void bakedInOpenWarnsBeforeRunning();
    void optionsPersistBetweenRuns();
    void commandDisabledWithoutActions();

private:
    // Dispatch Batch… and run `drive` on the dialog once it is modal.
    bool driveBatchDialog(const std::function<void(pictura::BatchDialog&)>& drive);
    QPair<QString, QString> firstAction() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::ActionsPanel* panel_ = nullptr;
    QTemporaryDir dir_;
};

void BatchDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    panel_ = window_->actionsPanel();
    QVERIFY(panel_);
    panel_->setPersistEnabledForTest(false);
    window_->batch().setMessageHandler([](const QString&, const QString&) {});
}

bool BatchDialogTest::driveBatchDialog(const std::function<void(pictura::BatchDialog&)>& drive)
{
    bool seen = false;
    int attempts = 0;
    QTimer poll;
    poll.setInterval(10);
    connect(&poll, &QTimer::timeout, this, [&] {
        pictura::BatchDialog* dialog = nullptr;
        for (auto* candidate : window_->findChildren<pictura::BatchDialog*>()) {
            if (candidate->isVisible()) {
                dialog = candidate;
            }
        }
        if (!dialog) {
            if (++attempts > 500) {
                poll.stop();
            }
            return;
        }
        poll.stop();
        seen = true;
        drive(*dialog);
        if (dialog->isVisible()) {
            dialog->reject();
        }
    });
    poll.start();
    window_->registry()->dispatch(QStringLiteral("file.automate.batch"));
    poll.stop();
    return seen;
}

QPair<QString, QString> BatchDialogTest::firstAction() const
{
    for (const auto& set : panel_->actionLibraryNames()) {
        if (!set.second.isEmpty()) {
            return qMakePair(set.first, set.second.first());
        }
    }
    return {};
}

void BatchDialogTest::commandOpensTheDialog()
{
    QVERIFY(!firstAction().first.isEmpty());
    window_->registry()->refresh();
    QAction* action = window_->registry()->action(QStringLiteral("file.automate.batch"));
    QVERIFY(action);
    QVERIFY(action->isEnabled());
    QVERIFY(driveBatchDialog([](pictura::BatchDialog& dialog) {
        QCOMPARE(dialog.windowTitle(), QStringLiteral("Batch"));
    }));
}

void BatchDialogTest::dialogShowsEveryGroup()
{
    pictura::BatchDialog dialog;
    dialog.optionsWidget()->setLibrary({{QStringLiteral("Set"), {QStringLiteral("A")}}});
    QCOMPARE(comboItems(child<QComboBox>(dialog, "batchSet")), QStringList{QStringLiteral("Set")});
    QCOMPARE(comboItems(child<QComboBox>(dialog, "batchAction")), QStringList{QStringLiteral("A")});
    QCOMPARE(comboItems(child<QComboBox>(dialog, "batchSource")),
             (QStringList{QStringLiteral("Folder"), QStringLiteral("Opened Files")}));
    QCOMPARE(comboItems(child<QComboBox>(dialog, "batchDestination")),
             (QStringList{QStringLiteral("None"), QStringLiteral("Save and Close"),
                          QStringLiteral("Folder")}));
    QCOMPARE(comboItems(child<QComboBox>(dialog, "batchErrors")),
             (QStringList{QStringLiteral("Stop For Errors"), QStringLiteral("Log Errors To File")}));
    for (const char* name : {"batchOverrideOpen", "batchIncludeSubfolders", "batchSuppressOpen",
                             "batchSuppressProfile", "batchOverrideSave", "compatWindows",
                             "compatMac", "compatUnix"}) {
        QVERIFY2(child<QCheckBox>(dialog, name), name);
    }
    for (const char* name : {"batchSourceChoose", "batchDestinationChoose", "batchErrorLogChoose"}) {
        QVERIFY2(child<QPushButton>(dialog, name), name);
    }
    QStringList tokens{QString()};
    tokens << pictura::batchTokenLabels();
    for (int i = 0; i < pictura::FileNamingWidget::kFieldCount; ++i) {
        auto* field = dialog.findChild<QComboBox*>(QStringLiteral("namingField%1").arg(i));
        QVERIFY(field);
        QVERIFY(field->isEditable());
        QCOMPARE(comboItems(field), tokens);
    }
    QCOMPARE(child<QSpinBox>(dialog, "startingSerial")->minimum(), 1);
    QCheckBox* unixBox = child<QCheckBox>(dialog, "compatUnix");
    QVERIFY(unixBox->isChecked());
    QVERIFY(!unixBox->isEnabled());
    // CS6's default scheme.
    QCOMPARE(dialog.options().namingFields,
             (QStringList{QStringLiteral("Document Name"), QStringLiteral("extension")}));
    // The fields read left to right, then top to bottom.
    auto* grid = qobject_cast<QGridLayout*>(
        child<QComboBox>(dialog, "namingField0")->parentWidget()->layout()->itemAt(1)->layout());
    QVERIFY(grid);
    for (int i = 0; i < pictura::FileNamingWidget::kFieldCount; ++i) {
        int row = -1, column = -1, rowSpan = 0, columnSpan = 0;
        grid->getItemPosition(grid->indexOf(dialog.findChild<QComboBox*>(
                                  QStringLiteral("namingField%1").arg(i))),
                              &row, &column, &rowSpan, &columnSpan);
        QCOMPARE(row, i / 2);
        QCOMPARE(column, i % 2);
    }
}

void BatchDialogTest::controlsFollowSourceAndDestination()
{
    pictura::BatchDialog dialog;
    child<QComboBox>(dialog, "batchSource")->setCurrentIndex(0);
    child<QComboBox>(dialog, "batchDestination")->setCurrentIndex(2);
    QVERIFY(child<QPushButton>(dialog, "batchSourceChoose")->isEnabled());
    QVERIFY(child<QCheckBox>(dialog, "batchIncludeSubfolders")->isEnabled());
    QVERIFY(child<QPushButton>(dialog, "batchDestinationChoose")->isEnabled());
    QVERIFY(child<QCheckBox>(dialog, "batchOverrideSave")->isEnabled());
    QVERIFY(child<QWidget>(dialog, "batchNaming")->isEnabled());

    child<QComboBox>(dialog, "batchSource")->setCurrentIndex(1);
    child<QComboBox>(dialog, "batchDestination")->setCurrentIndex(0);
    QVERIFY(!child<QPushButton>(dialog, "batchSourceChoose")->isEnabled());
    QVERIFY(!child<QCheckBox>(dialog, "batchIncludeSubfolders")->isEnabled());
    QVERIFY(!child<QPushButton>(dialog, "batchDestinationChoose")->isEnabled());
    QVERIFY(!child<QCheckBox>(dialog, "batchOverrideSave")->isEnabled());
    QVERIFY(!child<QWidget>(dialog, "batchNaming")->isEnabled());

    child<QComboBox>(dialog, "batchDestination")->setCurrentIndex(1);
    QVERIFY(child<QCheckBox>(dialog, "batchOverrideSave")->isEnabled());
    QVERIFY(!child<QWidget>(dialog, "batchNaming")->isEnabled());
    QVERIFY(!child<QPushButton>(dialog, "batchErrorLogChoose")->isEnabled());
    child<QComboBox>(dialog, "batchErrors")->setCurrentIndex(1);
    QVERIFY(child<QPushButton>(dialog, "batchErrorLogChoose")->isEnabled());
}

void BatchDialogTest::exampleFollowsTheScheme()
{
    pictura::FileNamingWidget naming;
    naming.setNaming({QStringLiteral("DOCUMENT NAME"), QStringLiteral("1 Digit Serial Number"),
                      QStringLiteral("extension")},
                     3, false, false);
    QCOMPARE(naming.exampleText(), QStringLiteral("Example: MYFILE3.gif"));
}

void BatchDialogTest::invalidOptionsKeepTheDialogOpen()
{
    const auto action = firstAction();
    pictura::BatchDialog dialog(window_.get());
    dialog.optionsWidget()->setLibrary(panel_->actionLibraryNames());
    pictura::BatchOptions options;
    options.set = action.first;
    options.action = action.second;
    options.sourceFolder = dir_.path();
    options.destination = pictura::BatchOptions::Destination::Folder;
    dialog.optionsWidget()->setOptions(options);
    dialog.setValidator([this](const pictura::BatchOptions& chosen) {
        return panel_->validateBatch(chosen.toJson());
    });
    QString shown;
    dialog.setMessageHandler([&shown](const QString& text) { shown = text; });
    dialog.show();
    dialog.accept();
    QVERIFY(!shown.isEmpty());
    QVERIFY(dialog.isVisible());
    QVERIFY(dialog.result() != QDialog::Accepted);
    QVERIFY(!window_->batch().running());
}

void BatchDialogTest::bakedInOpenWarnsBeforeRunning()
{
    pictura::ActionsEngineApi* engine = panel_->engineForTest();
    const QString setId = engine->createSet(QStringLiteral("Open Warning"));
    const QString actionId =
        engine->createAction(setId, QStringLiteral("Opens"), 0, false, false, 0);
    QVERIFY(engine->startRecording(actionId, -1));
    engine->recordCommand(QStringLiteral("file.open"),
                          QJsonObject{{QStringLiteral("path"), dir_.filePath(QStringLiteral("x.png"))}});
    engine->stopRecording();
    panel_->refresh();

    pictura::BatchDialog dialog(window_.get());
    dialog.optionsWidget()->setLibrary(panel_->actionLibraryNames());
    pictura::BatchOptions options;
    options.set = QStringLiteral("Open Warning");
    options.action = QStringLiteral("Opens");
    options.sourceFolder = dir_.path();
    dialog.optionsWidget()->setOptions(options);
    dialog.setValidator([this](const pictura::BatchOptions& chosen) {
        return panel_->validateBatch(chosen.toJson());
    });
    dialog.setOpenProbe([this](const pictura::BatchOptions& chosen) {
        return panel_->actionReachesCommand(chosen.set, chosen.action, QStringLiteral("file.open"));
    });
    int warnings = 0;
    dialog.setConfirmHandler([&warnings](const QString&) {
        ++warnings;
        return false;
    });
    dialog.show();
    dialog.accept();
    QCOMPARE(warnings, 1);
    QVERIFY(dialog.isVisible());

    // With the override on there is nothing to warn about.
    child<QCheckBox>(dialog, "batchOverrideOpen")->setChecked(true);
    dialog.accept();
    QCOMPARE(warnings, 1);
    QCOMPARE(dialog.result(), int(QDialog::Accepted));
}

void BatchDialogTest::optionsPersistBetweenRuns()
{
    const auto action = firstAction();
    QVERIFY(driveBatchDialog([this, action](pictura::BatchDialog& dialog) {
        pictura::BatchOptions options;
        options.set = action.first;
        options.action = action.second;
        options.sourceFolder = dir_.path();
        options.includeSubfolders = true;
        options.startingSerial = 5;
        dialog.optionsWidget()->setOptions(options);
        dialog.accept();
    }));
    QTRY_VERIFY(!window_->batch().running());

    pictura::BatchOptions shown;
    QVERIFY(driveBatchDialog([&shown](pictura::BatchDialog& dialog) { shown = dialog.options(); }));
    QCOMPARE(shown.sourceFolder, dir_.path());
    QVERIFY(shown.includeSubfolders);
    QCOMPARE(shown.startingSerial, 5);
}

void BatchDialogTest::commandDisabledWithoutActions()
{
    pictura::ActionsEngineApi* engine = panel_->engineForTest();
    QVERIFY(engine->clearAll());
    panel_->refresh();
    window_->registry()->refresh();
    QVERIFY(!window_->registry()->action(QStringLiteral("file.automate.batch"))->isEnabled());
    QVERIFY(engine->reset(false));
    panel_->refresh();
    window_->registry()->refresh();
    QVERIFY(window_->registry()->action(QStringLiteral("file.automate.batch"))->isEnabled());
}

QTEST_MAIN(BatchDialogTest)
#include "tst_batch_dialog.moc"
