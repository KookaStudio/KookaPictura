// File > Place's multi-file sequence: one Free Transform per file, the
// cancel/continue rules, and the per-batch failure summary.

#include <QtTest/QtTest>

#include <optional>

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QVector>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>

#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "api/paint_bridge.h"
#include "api/view_bridge.h"
#include "color_settings_dialog.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "frame_place.h"
#include "image_view.h"
#include "profile_warning_dialog.h"
#include "tools.h"

#include "multi_file_test_support.h"
#include "qt_test_support.h"

using namespace pictura;

namespace {

template <typename T>
T* findVisible(const QString& name)
{
    for (QWidget* widget : QApplication::allWidgets()) {
        auto* dialog = qobject_cast<T*>(widget);
        if (dialog && dialog->isVisible() && dialog->objectName() == name) {
            return dialog;
        }
    }
    return nullptr;
}

PictureView* makeTarget(PicturaMainWindow& window,
                        const QString& name = QStringLiteral("PlaceTarget"))
{
    if (!window.newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window.activeView();
}

bool writeText(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

bool writePsd(PicturaMainWindow& window, const QString& path)
{
    if (!window.newDocument(QStringLiteral("PlaceSource"), 8, 8, QStringLiteral("rgb"), 8,
                            QStringLiteral("white"))) {
        return false;
    }
    PictureView* view = window.activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(30, 120, 220).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    const bool saved = window.saveActiveAs(path);
    window.closeDocument(window.activeDocumentIndex(), false);
    return saved;
}

QString writeAdobeJpeg(PicturaMainWindow& window, const QString& path)
{
    if (!window.newDocument(QStringLiteral("ProfileSource"), 8, 8, QStringLiteral("rgb"), 8,
                            QStringLiteral("white"))) {
        return {};
    }
    PictureView* view = window.activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(255, 0, 0).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    if (!view->assign_profile(1)) {
        window.closeDocument(window.activeDocumentIndex(), false);
        return {};
    }
    pictura::set_save_embed_profile(*view, true);
    const bool saved = view->save(path);
    window.closeDocument(window.activeDocumentIndex(), false);
    return saved ? path : QString();
}

void setAsks(PicturaMainWindow& window, bool mismatch, bool missing)
{
    auto* timer = new QTimer(&window);
    timer->setInterval(10);
    QObject::connect(timer, &QTimer::timeout, &window, [timer, mismatch, missing]() {
        auto* dialog = findVisible<ColorSettingsDialog>(QStringLiteral("colorSettingsDialog"));
        if (!dialog) {
            return;
        }
        timer->stop();
        if (auto* box =
                dialog->findChild<QCheckBox*>(QStringLiteral("colorSettingsAskMismatch"))) {
            box->setChecked(mismatch);
        }
        if (auto* box =
                dialog->findChild<QCheckBox*>(QStringLiteral("colorSettingsAskMissing"))) {
            box->setChecked(missing);
        }
        dialog->accept();
        timer->deleteLater();
    });
    timer->start();
    QTimer::singleShot(5000, timer, [timer]() {
        auto* dialog = findVisible<ColorSettingsDialog>(QStringLiteral("colorSettingsDialog"));
        if (dialog) dialog->reject();
        timer->deleteLater();
    });
    window.showColorSettings();
    timer->stop();
    timer->deleteLater();
}

struct ScopedTransformPrompt {
    ScopedTransformPrompt() { pictura::setTransformPromptInteractive(true); }
    ~ScopedTransformPrompt() { pictura::setTransformPromptInteractive(false); }
};

enum class PlaceAction {
    CommitEnter,
    CommitApply,
    Escape,
    CancelControl,
    CancelPressOff,
    ToolSwitch,
    SwitchDocument,
    CloseDocument,
};

enum class ToolSwitchAnswer { DontApply, Cancel, Apply };

// Drives one File > Place batch through the real dialog, profile warnings, each
// transform session, the Cancel/continue prompt, and the summary. A watchdog
// cancels a stuck session and fails the test instead of hanging it.
struct PlaceDriver {
    PicturaMainWindow* window = nullptr;
    QStringList paths;
    QVector<QVector<PlaceAction>> plan;
    ToolSwitchAnswer toolSwitchAnswer = ToolSwitchAnswer::DontApply;
    bool cancelDialog = false;
    bool cancelProfile = false;
    bool cancelPromptContinue = true;
    int switchIndex = 0;

    OpenDialogProbe probe;
    QString error;
    QString reentryPath;
    QString summaryText;
    int fileDialogs = 0;
    int profileDialogs = 0;
    int toolPrompts = 0;
    int cancelPrompts = 0;
    int summaries = 0;
    int sessions = 0;
    int firstSessionRows = -1;
    int reentryDispatches = 0;
    int applyClicks = 0;
    int cancelClicks = 0;
    bool timedOut = false;

    QTimer timer;
    QTimer watchdog;
    bool busy = false;
    int sessionStarts = 0;
    bool wasActive = false;
    int actionIndex = 0;

    void start()
    {
        QObject::connect(&timer, &QTimer::timeout, &timer, [this]() { tick(); });
        timer.start(10);
        if (auto* tools = window ? window->findChild<ToolController*>() : nullptr) {
            QObject::connect(tools, &ToolController::transformChanged, &timer, [this]() {
                PictureView* view = window ? window->activeView() : nullptr;
                const bool active = view && view->transform_session_active();
                if (active && !wasActive) {
                    ++sessionStarts;
                }
                wasActive = active;
            });
        }
        QObject::connect(&watchdog, &QTimer::timeout, &watchdog, [this]() {
            timedOut = true;
            if (window) {
                if (PictureView* view = window->activeView();
                    view && view->transform_session_active()) {
                    view->cancel_transform();
                }
            }
            if (auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                box->reject();
            }
            auto* warning = findVisible<ProfileWarningDialog>(
                QStringLiteral("profileWarningDialog"));
            if (warning) warning->reject();
            if (QFileDialog* dialog = visibleFileDialog()) {
                dialog->reject();
            }
        });
        watchdog.start(10000);
    }

    void stop()
    {
        timer.stop();
        watchdog.stop();
    }

    void tick()
    {
        if (busy) {
            return;
        }
        busy = true;
        handle();
        busy = false;
    }

    void handle()
    {
        if (QFileDialog* dialog = visibleFileDialog()) {
            ++fileDialogs;
            probe.seen = true;
            probe.mode = dialog->fileMode();
            probe.filters = dialog->nameFilters();
            if (cancelDialog) {
                dialog->reject();
            } else {
                chooseFiles(dialog, paths, &probe);
                if (!probe.error.isEmpty()) {
                    error = probe.error;
                }
            }
            return;
        }
        if (auto* warning = findVisible<ProfileWarningDialog>(
                QStringLiteral("profileWarningDialog"))) {
            ++profileDialogs;
            if (cancelProfile) {
                warning->reject();
            } else {
                warning->accept();
            }
            return;
        }
        if (auto* box = findVisible<QMessageBox>(QStringLiteral("placeCancelDialog"))) {
            ++cancelPrompts;
            if (!reentryPath.isEmpty()) {
                pictura::placeDocuments(*window, {reentryPath}); ++reentryDispatches;
            }
            const QString name = cancelPromptContinue
                ? QStringLiteral("placeCancelContinue")
                : QStringLiteral("placeCancelWhole");
            if (auto* button = box->findChild<QAbstractButton*>(name)) {
                button->click();
            } else {
                box->reject();
            }
            return;
        }
        if (auto* box = findVisible<QMessageBox>(QStringLiteral("placeSummaryDialog"))) {
            ++summaries;
            summaryText = box->text();
            box->accept();
            return;
        }
        if (auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
            box && box->text().contains(QStringLiteral("Apply the transformation?"))) {
            ++toolPrompts;
            if (toolSwitchAnswer == ToolSwitchAnswer::Cancel) {
                QKeyEvent escape(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
                QCoreApplication::sendEvent(box, &escape);
            } else {
                const QString wanted = toolSwitchAnswer == ToolSwitchAnswer::DontApply
                    ? QStringLiteral("Don't Apply")
                    : QStringLiteral("Apply");
                QAbstractButton* button = nullptr;
                for (QAbstractButton* candidate : box->buttons()) {
                    if (candidate->text() == wanted) {
                        button = candidate;
                    }
                }
                if (button) {
                    button->click();
                } else {
                    box->reject();
                }
            }
            return;
        }
        handleTransform();
    }

    bool handleTransform()
    {
        PictureView* view = window ? window->activeView() : nullptr;
        if (!view || !view->transform_session_active()) {
            return false;
        }
        if (sessions != sessionStarts) {
            sessions = sessionStarts;
            actionIndex = 0;
            if (sessions == 1) {
                firstSessionRows = layers_api::rowCount(*view);
            }
        }
        if (sessions <= 0 || sessions > plan.size()) {
            error = QStringLiteral("unexpected transform session");
            return true;
        }
        const QVector<PlaceAction>& actions = plan.at(sessions - 1);
        if (actionIndex >= actions.size()) {
            error = QStringLiteral("no action for transform session");
            return true;
        }
        act(actions.at(actionIndex++));
        return true;
    }

    void act(PlaceAction action)
    {
        switch (action) {
        case PlaceAction::CommitEnter: {
            if (ImageView* canvas = window->imageView()) {
                QKeyEvent key(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
                QCoreApplication::sendEvent(canvas, &key);
            }
            break;
        }
        case PlaceAction::CommitApply: {
            auto* button = window->findChild<QToolButton*>(QStringLiteral("optionsTransformApply"));
            if (!button) {
                error = QStringLiteral("no apply control");
                break;
            }
            ++applyClicks;
            button->click();
            break;
        }
        case PlaceAction::Escape: {
            if (ImageView* canvas = window->imageView()) {
                QKeyEvent key(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
                QCoreApplication::sendEvent(canvas, &key);
            }
            break;
        }
        case PlaceAction::CancelControl: {
            auto* button =
                window->findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"));
            if (!button) {
                error = QStringLiteral("no Cancel control");
                break;
            }
            ++cancelClicks;
            button->click();
            break;
        }
        case PlaceAction::CancelPressOff: {
            auto* button =
                window->findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"));
            if (!button) {
                error = QStringLiteral("no Cancel control");
                break;
            }
            // A press released outside the control emits no clicked(): the
            // session stays active and the next action commits it.
            QTest::mousePress(button, Qt::LeftButton);
            QTest::mouseRelease(button, Qt::LeftButton, Qt::NoModifier, QPoint(-10, -10));
            break;
        }
        case PlaceAction::ToolSwitch:
            // Deferred so this tick returns and the driver can answer the
            // modal "Apply the transformation?" prompt.
            QTimer::singleShot(0, window, [this]() {
                window->setActiveTool(ToolId::Brush);
            });
            break;
        case PlaceAction::SwitchDocument:
            window->setActiveDocumentIndex(switchIndex);
            break;
        case PlaceAction::CloseDocument:
            window->closeDocument(window->activeDocumentIndex(), false);
            break;
        }
    }
};

void runPlace(PicturaMainWindow& window, PlaceDriver& driver)
{
    driver.window = &window;
    driver.start();
    window.registry()->dispatch(QString::fromLatin1(command_ids::FilePlace));
    driver.stop();
    QVERIFY2(!driver.timedOut, "the Place driver timed out");
}

} // namespace

class PlaceSequenceTest : public QObject {
    Q_OBJECT

private slots:
    void multiSelectPlacesOneLayerPerFile();
    void eachPlaceWaitsForThePreviousTransform();
    void mixedFormatsPlacePerFile();
    void applyControlCommitsAndAdvances();
    void failedFileDoesNotStopTheRestAndIsSummarized();
    void allFilesFailShowsOneSummary();
    void cancelledProfileWarningSkipsWithoutSummary();
    void placeUnavailableWithoutDocument();
    void placeUnavailableDuringTransform();
    void failedPlaceDoesNotReuseTheProfilePolicy();
    void emptySelectionStartsNoBatch();
    void dispatchDuringBatchIsIgnored();
    void placeDialogOffersImagesFilter();
    void escRemovesThePlacementAndHistory();
    void escContinuePlacesTheNextFile();
    void escCancelWholeStopsTheBatch();
    void escOnLastItemEndsWithoutDialog();
    void cancelControlRemovesThePlacement();
    void cancelPressDragOffKeepsThePlacement();
    void escapeInToolSwitchPromptDoesNotCancelThePlace();
    void toolSwitchDontApplyKeepsThePlacement();
    void identityCommitAdvancesWithoutHistory();
    void documentSwitchAbortsTheBatch();
    void closingDocumentAbortsTheBatch();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void PlaceSequenceTest::multiSelectPlacesOneLayerPerFile()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.fileDialogs, 1);
    QVERIFY(driver.probe.seen);
    QCOMPARE(int(driver.probe.mode), int(QFileDialog::ExistingFiles));
    QCOMPARE(int(driver.probe.selected.size()), 2);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
    QCOMPARE(target->history_label(baseHistory + 1), QStringLiteral("Place"));
    QCOMPARE(target->active_layer_path(), layers_api::rowAt(*target, 0).path);
}

void PlaceSequenceTest::eachPlaceWaitsForThePreviousTransform()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.firstSessionRows, baseRows + 1);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
}

void PlaceSequenceTest::mixedFormatsPlacePerFile()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString psd = dir.filePath(QStringLiteral("a.psd"));
    const QString png = dir.filePath(QStringLiteral("b.png"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(writePsd(*window, psd));
    QVERIFY(writePng(png, Qt::blue));
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {psd, png};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
    const QString pngLayer = layers_api::rowAt(*target, 0).path;
    const QString psdLayer = layers_api::rowAt(*target, 1).path;
    QVERIFY(target->layer_smart_object_state(pngLayer).startsWith(QStringLiteral("embedded:")));
    QVERIFY(target->layer_smart_object_state(psdLayer).startsWith(QStringLiteral("embedded:")));

    const QString exported = dir.filePath(QStringLiteral("exported.psd"));
    QVERIFY(target->export_smart_object_contents(psdLayer, exported));
    auto readBytes = [](const QString& path) {
        QFile file(path);
        return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
    };
    QVERIFY(!readBytes(exported).isEmpty());
    QCOMPARE(readBytes(exported), readBytes(psd));
}

void PlaceSequenceTest::applyControlCommitsAndAdvances()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitApply},
                                                {PlaceAction::CommitApply}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.applyClicks, 2);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
}

void PlaceSequenceTest::failedFileDoesNotStopTheRestAndIsSummarized()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString bad = dir.filePath(QStringLiteral("bad.png"));
    const QString last = dir.filePath(QStringLiteral("z.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writeText(bad, QByteArrayLiteral("not an image")));
    QVERIFY(writePng(last, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, bad, last};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 1);
    QVERIFY2(driver.summaryText.contains(QFileInfo(bad).fileName()),
             qPrintable(driver.summaryText));
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
}

void PlaceSequenceTest::allFilesFailShowsOneSummary()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writeText(first, QByteArrayLiteral("not an image")));
    QVERIFY(writeText(second, QByteArrayLiteral("still not an image")));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 0);
    QCOMPARE(driver.summaries, 1);
    QVERIFY2(driver.summaryText.contains(QFileInfo(first).fileName()),
             qPrintable(driver.summaryText));
    QVERIFY2(driver.summaryText.contains(QFileInfo(second).fileName()),
             qPrintable(driver.summaryText));
    QCOMPARE(layers_api::rowCount(*target), baseRows);
    QCOMPARE(target->history_count(), baseHistory);
}

void PlaceSequenceTest::cancelledProfileWarningSkipsWithoutSummary()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString adobe = dir.filePath(QStringLiteral("adobe.jpg"));
    const QString good = dir.filePath(QStringLiteral("good.png"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(!writeAdobeJpeg(*window, adobe).isEmpty());
    QVERIFY(writePng(good, Qt::blue));
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();
    setAsks(*window, true, false);

    PlaceDriver driver;
    driver.paths = {adobe, good};
    driver.cancelProfile = true;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.profileDialogs, 1);
    QCOMPARE(driver.sessions, 1);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
    QCOMPARE(target->history_count(), baseHistory + 1);
}

void PlaceSequenceTest::placeUnavailableWithoutDocument()
{
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    window->registry()->refresh();
    QAction* action = window->registry()->action(QString::fromLatin1(command_ids::FilePlace));
    QVERIFY(action != nullptr);
    QVERIFY(!action->isEnabled());

    window->registry()->dispatch(QString::fromLatin1(command_ids::FilePlace));
    QVERIFY(visibleFileDialog() == nullptr);
    QCOMPARE(window->documentCount(), 0);
}

void PlaceSequenceTest::placeUnavailableDuringTransform()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("a.png"));
    QVERIFY(writePng(png, Qt::red));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const QString placed = target->place_image(png);
    QVERIFY(!placed.isEmpty());
    QVERIFY(window->beginFreeTransform(placed));
    QVERIFY(target->transform_session_active());

    QAction* action = window->registry()->action(QString::fromLatin1(command_ids::FilePlace));
    QVERIFY(action != nullptr);
    QVERIFY(!action->isEnabled());
    const int rows = layers_api::rowCount(*target);
    window->registry()->dispatch(QString::fromLatin1(command_ids::FilePlace));
    QVERIFY(visibleFileDialog() == nullptr);
    QCOMPARE(layers_api::rowCount(*target), rows);
    target->cancel_transform();
}

void PlaceSequenceTest::failedPlaceDoesNotReuseTheProfilePolicy()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString adobe = dir.filePath(QStringLiteral("adobe.jpg"));
    const QString missing = dir.filePath(QStringLiteral("missing.png"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(!writeAdobeJpeg(*window, adobe).isEmpty());

    auto sampleWith = [&](const QString& name, int policy,
                          bool failFirst) -> std::optional<QRgb> {
        if (!window->newDocument(name, 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white"))) {
            return std::nullopt;
        }
        PictureView* view = window->activeView();
        if (policy >= 0) {
            pictura::set_open_policy_override(*view, policy);
        }
        if (failFirst && !view->place_image(missing).isEmpty()) {
            return std::nullopt;
        }
        if (view->place_image(adobe).isEmpty()) {
            return std::nullopt;
        }
        const QRgb sample = view->sample_argb(2, 2);
        window->closeDocument(window->activeDocumentIndex(), false);
        return sample;
    };

    const auto baseline = sampleWith(QStringLiteral("Baseline"), -1, false);
    const auto off = sampleWith(QStringLiteral("Off"), 2, false);
    const auto leaked = sampleWith(QStringLiteral("Leak"), 2, true);
    QVERIFY(baseline.has_value());
    QVERIFY(off.has_value());
    QVERIFY(leaked.has_value());
    QCOMPARE(*leaked, *baseline);
    QVERIFY(*leaked != *off);
}

void PlaceSequenceTest::emptySelectionStartsNoBatch()
{
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.cancelDialog = true;
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.fileDialogs, 1);
    QCOMPARE(driver.sessions, 0);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows);
    QCOMPARE(target->history_count(), baseHistory);
}

void PlaceSequenceTest::dispatchDuringBatchIsIgnored()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.reentryPath = second;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::Escape},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.reentryDispatches, 1);
    QCOMPARE(driver.fileDialogs, 1);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
}

void PlaceSequenceTest::placeDialogOffersImagesFilter()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("a.png"));
    QVERIFY(writePng(png, Qt::red));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);

    PlaceDriver driver;
    driver.paths = {png};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    bool images = false;
    for (const QString& filter : driver.probe.filters) {
        if (filter.startsWith(QStringLiteral("Images ("))) {
            images = true;
        }
    }
    QVERIFY2(images, qPrintable(driver.probe.filters.join(QStringLiteral(" | "))));
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
}

void PlaceSequenceTest::escRemovesThePlacementAndHistory()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("a.png"));
    QVERIFY(writePng(png, Qt::red));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {png};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::Escape}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 1);
    QCOMPARE(driver.cancelPrompts, 0);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows);
    QCOMPARE(target->history_index(), baseHistory - 1);
}

void PlaceSequenceTest::escContinuePlacesTheNextFile()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.cancelPromptContinue = true;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::Escape},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.cancelPrompts, 1);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
    QCOMPARE(target->history_index(), baseHistory);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
}

void PlaceSequenceTest::escCancelWholeStopsTheBatch()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.cancelPromptContinue = false;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::Escape}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.cancelPrompts, 1);
    QCOMPARE(driver.sessions, 1);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows);
    QCOMPARE(target->history_index(), baseHistory - 1);
}

void PlaceSequenceTest::escOnLastItemEndsWithoutDialog()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::Escape}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.cancelPrompts, 0);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
    QCOMPARE(target->history_index(), baseHistory);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
}

void PlaceSequenceTest::cancelControlRemovesThePlacement()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("a.png"));
    QVERIFY(writePng(png, Qt::red));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {png};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CancelControl}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.cancelClicks, 1);
    QCOMPARE(driver.cancelPrompts, 0);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows);
    QCOMPARE(target->history_index(), baseHistory - 1);
}

void PlaceSequenceTest::cancelPressDragOffKeepsThePlacement()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{
        {PlaceAction::CancelPressOff, PlaceAction::CommitEnter},
        {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.cancelPrompts, 0);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
}

void PlaceSequenceTest::escapeInToolSwitchPromptDoesNotCancelThePlace()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    ScopedTransformPrompt prompt;
    PlaceDriver driver;
    driver.paths = {first, second};
    driver.toolSwitchAnswer = ToolSwitchAnswer::Cancel;
    driver.plan = QVector<QVector<PlaceAction>>{
        {PlaceAction::ToolSwitch, PlaceAction::CommitEnter},
        {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.toolPrompts, 1);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
}

void PlaceSequenceTest::toolSwitchDontApplyKeepsThePlacement()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    ScopedTransformPrompt prompt;
    PlaceDriver driver;
    driver.paths = {first, second};
    driver.toolSwitchAnswer = ToolSwitchAnswer::DontApply;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::ToolSwitch},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.toolPrompts, 1);
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
}

void PlaceSequenceTest::identityCommitAdvancesWithoutHistory()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CommitEnter},
                                                {PlaceAction::CommitEnter}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 2);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 2);
    QCOMPARE(target->history_count(), baseHistory + 2);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
    QCOMPARE(target->history_label(baseHistory + 1), QStringLiteral("Place"));
}

void PlaceSequenceTest::documentSwitchAbortsTheBatch()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Other"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* other = window->activeView();
    QVERIFY(other != nullptr);
    PictureView* target = makeTarget(*window);
    QVERIFY(target != nullptr);
    const int otherRows = layers_api::rowCount(*other);
    const int baseRows = layers_api::rowCount(*target);
    const int baseHistory = target->history_count();

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.switchIndex = 0;
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::SwitchDocument}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 1);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(window->activeDocumentIndex(), 0);
    QCOMPARE(layers_api::rowCount(*target), baseRows + 1);
    QCOMPARE(target->history_label(baseHistory), QStringLiteral("Place"));
    QCOMPARE(layers_api::rowCount(*other), otherRows);
}

void PlaceSequenceTest::closingDocumentAbortsTheBatch()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("a.png"));
    const QString second = dir.filePath(QStringLiteral("b.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Other"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* other = window->activeView();
    QVERIFY(other != nullptr);
    const int otherRows = layers_api::rowCount(*other);
    QVERIFY(makeTarget(*window) != nullptr);

    PlaceDriver driver;
    driver.paths = {first, second};
    driver.plan = QVector<QVector<PlaceAction>>{{PlaceAction::CloseDocument}};
    runPlace(*window, driver);

    QVERIFY2(driver.error.isEmpty(), qPrintable(driver.error));
    QCOMPARE(driver.sessions, 1);
    QCOMPARE(driver.summaries, 0);
    QCOMPARE(window->documentCount(), 1);
    QCOMPARE(window->activeDocumentIndex(), 0);
    QCOMPARE(layers_api::rowCount(*other), otherRows);
}

QTEST_MAIN(PlaceSequenceTest)
#include "tst_place_sequence.moc"
