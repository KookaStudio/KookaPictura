// File > Scripts > Statistics…: the Image Statistics dialog, the stack merge,
// failure handling, remembered choices, and the command's edition gate.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "session.h"
#include "statistics_dialog.h"

#include "api/layers_api.h"
#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

#include "qt_test_support.h"

using pictura::StatisticsDialog;

namespace {

template <typename T>
T* child(QWidget& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

QDialog* dialogNamed(const QString& name)
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<QDialog*>(widget); dialog && dialog->objectName() == name) {
            return dialog;
        }
    }
    return nullptr;
}

bool nearChannel(int value, int expected)
{
    return qAbs(value - expected) <= 2;
}

} // namespace

class StatisticsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void statisticsCommandEnabledWithoutDocument();
    void dialogShowsControls();
    void dialogDefaults();
    void twoFilesUnlockOkAndAlign();
    void buttonsFollowSelectionAndOpenDocuments();
    void addFilesKeepsOrderAndDedupes();
    void folderAddsSupportedImages();
    void removeSelected();
    void addOpenFilesAddsSavedOnly();
    void rememberedChoiceSeedsDialog();
    void cancelChangesNothing();
    void mergesTwoImagesWithMean();
    void selectedStackModeIsApplied();
    void unreadableSourceIsSkipped();
    void fewerThanTwoDecodableCreatesNothing();
    void acceptedChoiceIsStored();
    void rememberedChoiceSeedsTheNextInvocation();
    void worksWithoutDocument();

private:
    QString makeImage(const QString& name, int width, int height, const QColor& color);
    QString makeCorrupt(const QString& name);
    bool runStatistics(const QStringList& paths, int mode, bool align,
                       QString* warning = nullptr);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void StatisticsTest::initTestCase()
{
    qputenv("PICTURA_EDITION", "extended");
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void StatisticsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

QString StatisticsTest::makeImage(const QString& name, int width, int height,
                                  const QColor& color)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(color);
    const QString path = dir_.filePath(name);
    return image.save(path, "PNG") ? path : QString();
}

QString StatisticsTest::makeCorrupt(const QString& name)
{
    const QString path = dir_.filePath(name);
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return QString();
    }
    file.write("not an image");
    file.close();
    return path;
}

bool StatisticsTest::runStatistics(const QStringList& paths, int mode, bool align,
                                   QString* warning)
{
    auto opened = std::make_shared<bool>(false);
    QString warningText;
    QTimer closer;
    closer.setInterval(10);
    connect(&closer, &QTimer::timeout, this, [&warningText] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* box = qobject_cast<QMessageBox*>(widget)) {
                warningText = box->text();
                box->accept();
            }
        }
    });
    closer.start();

    QTimer::singleShot(0, this, [this, paths, mode, align, opened] {
        auto* dialog = qobject_cast<StatisticsDialog*>(
            dialogNamed(QStringLiteral("statisticsDialog")));
        if (!dialog) {
            return;
        }
        *opened = true;
        dialog->addFiles(paths);
        if (auto* combo = child<QComboBox>(*dialog, "statisticsStackMode")) {
            const int index = combo->findData(mode);
            if (index >= 0) {
                combo->setCurrentIndex(index);
            }
        }
        if (auto* box = child<QCheckBox>(*dialog, "statisticsAlign")) {
            box->setChecked(align);
        }
        dialog->accept();
    });

    const bool dispatched = window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileScriptsStatistics));
    closer.stop();
    if (warning) {
        *warning = warningText;
    }
    return dispatched && *opened;
}

void StatisticsTest::statisticsCommandEnabledWithoutDocument()
{
    QCOMPARE(window_->documentCount(), 0);
    QAction* action = window_->registry()->action(
        QString::fromLatin1(pictura::command_ids::FileScriptsStatistics));
    QVERIFY(action != nullptr);
    QCOMPARE(action->text(), QStringLiteral("Statistics…"));
    QVERIFY(action->isEnabled());
    const QString id = QString::fromLatin1(pictura::command_ids::FileScriptsStatistics);
    QVERIFY(window_->registry()->isModal(id));
    bool found = false;
    for (const pictura::CommandInfo& info : window_->registry()->describe()) {
        if (info.id != id) {
            continue;
        }
        found = true;
        QCOMPARE(info.path, QStringList({QStringLiteral("File"), QStringLiteral("Scripts"),
                                         QStringLiteral("Statistics…")}));
        QCOMPARE(info.label, QStringLiteral("Statistics…"));
    }
    QVERIFY(found);
}

void StatisticsTest::dialogShowsControls()
{
    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Image Statistics"));
    QVERIFY(child<QGroupBox>(dialog, "statisticsSourceGroup"));
    QVERIFY(child<QComboBox>(dialog, "statisticsStackMode"));
    QVERIFY(child<QComboBox>(dialog, "statisticsUse"));
    QVERIFY(child<QListWidget>(dialog, "statisticsFiles"));
    QVERIFY(child<QPushButton>(dialog, "statisticsBrowse"));
    QVERIFY(child<QPushButton>(dialog, "statisticsRemove"));
    QVERIFY(child<QPushButton>(dialog, "statisticsAddOpen"));
    QVERIFY(child<QCheckBox>(dialog, "statisticsAlign"));
    QVERIFY(child<QPushButton>(dialog, "statisticsOk"));
    QVERIFY(child<QPushButton>(dialog, "statisticsCancel"));
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("statisticsSmartObject")) == nullptr);
    auto* mode = child<QComboBox>(dialog, "statisticsStackMode");
    QVERIFY(mode != nullptr);
    QCOMPARE(mode->count(), 11);
    QCOMPARE(mode->itemText(0), QStringLiteral("Mean"));
    QCOMPARE(mode->itemText(10), QStringLiteral("Entropy"));
    auto* use = child<QComboBox>(dialog, "statisticsUse");
    QVERIFY(use != nullptr);
    QCOMPARE(use->itemText(0), QStringLiteral("Files"));
    QCOMPARE(use->itemData(0).toString(), QStringLiteral("files"));
    QCOMPARE(use->itemText(1), QStringLiteral("Folder"));
    QCOMPARE(use->itemData(1).toString(), QStringLiteral("folder"));
    bool labelFound = false;
    for (QLabel* label : dialog.findChildren<QLabel*>()) {
        if (label->text().contains(QStringLiteral("Choose Stack Mode"))) {
            labelFound = true;
        }
    }
    QVERIFY(labelFound);
    QCOMPARE(child<QPushButton>(dialog, "statisticsBrowse")->text(), QStringLiteral("Browse…"));
    QCOMPARE(child<QPushButton>(dialog, "statisticsRemove")->text(), QStringLiteral("Remove"));
    QCOMPARE(child<QPushButton>(dialog, "statisticsAddOpen")->text(),
             QStringLiteral("Add Open Files"));
    QCOMPARE(child<QCheckBox>(dialog, "statisticsAlign")->text(),
             QStringLiteral("Attempt to Automatically Align Source Images"));
}

void StatisticsTest::dialogDefaults()
{
    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    auto* use = child<QComboBox>(dialog, "statisticsUse");
    QVERIFY(use != nullptr);
    QCOMPARE(use->currentData().toString(), QStringLiteral("files"));
    auto* mode = child<QComboBox>(dialog, "statisticsStackMode");
    QVERIFY(mode != nullptr);
    QCOMPARE(mode->currentData().toInt(), 3);
    auto* align = child<QCheckBox>(dialog, "statisticsAlign");
    QVERIFY(align != nullptr);
    QVERIFY(!align->isChecked());
    QCOMPARE(child<QListWidget>(dialog, "statisticsFiles")->count(), 0);
    QVERIFY(!child<QPushButton>(dialog, "statisticsOk")->isEnabled());

    StatisticsDialog invalid({}, false, -1, false, dir_.path());
    QCOMPARE(child<QComboBox>(invalid, "statisticsStackMode")->currentData().toInt(), 3);
}

void StatisticsTest::twoFilesUnlockOkAndAlign()
{
    const QString first = makeImage(QStringLiteral("first.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("second.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    auto* ok = child<QPushButton>(dialog, "statisticsOk");
    auto* align = child<QCheckBox>(dialog, "statisticsAlign");
    QVERIFY(ok != nullptr);
    QVERIFY(align != nullptr);
    dialog.addFiles({first});
    QVERIFY(!ok->isEnabled());
    QVERIFY(!align->isEnabled());
    dialog.addFiles({second});
    QVERIFY(ok->isEnabled());
    QVERIFY(align->isEnabled());
    align->setChecked(true);
    QVERIFY(dialog.alignImages());
}

void StatisticsTest::buttonsFollowSelectionAndOpenDocuments()
{
    const QString first = makeImage(QStringLiteral("select.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("select2.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());

    StatisticsDialog empty({}, false, 3, false, dir_.path());
    QVERIFY(!child<QPushButton>(empty, "statisticsAddOpen")->isEnabled());
    QVERIFY(!child<QPushButton>(empty, "statisticsRemove")->isEnabled());
    empty.addFiles({first, second});
    auto* list = child<QListWidget>(empty, "statisticsFiles");
    QVERIFY(list != nullptr);
    list->setCurrentRow(0);
    QVERIFY(child<QPushButton>(empty, "statisticsRemove")->isEnabled());

    StatisticsDialog withOpen({first}, false, 3, false, dir_.path());
    QVERIFY(child<QPushButton>(withOpen, "statisticsAddOpen")->isEnabled());
}

void StatisticsTest::addFilesKeepsOrderAndDedupes()
{
    const QString first = makeImage(QStringLiteral("order1.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("order2.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    dialog.addFiles({first, second});
    QCOMPARE(dialog.paths(), QStringList({first, second}));
    dialog.addFiles({second, first});
    QCOMPARE(dialog.paths().size(), 2);
    dialog.addFiles({dir_.filePath(QStringLiteral("missing.png"))});
    QCOMPARE(dialog.paths().size(), 2);
}

void StatisticsTest::folderAddsSupportedImages()
{
    QTemporaryDir folder;
    QVERIFY(folder.isValid());
    QImage image(4, 4, QImage::Format_RGB32);
    image.fill(Qt::red);
    QVERIFY(image.save(folder.filePath(QStringLiteral("one.png")), "PNG"));
    QVERIFY(image.save(folder.filePath(QStringLiteral("two.png")), "PNG"));
    QFile notes(folder.filePath(QStringLiteral("notes.txt")));
    QVERIFY(notes.open(QIODevice::WriteOnly));
    notes.write("not an image");
    notes.close();

    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    dialog.addFolder(folder.path());
    QCOMPARE(dialog.paths().size(), 2);
    for (const QString& path : dialog.paths()) {
        QVERIFY(path.endsWith(QLatin1String(".png")));
    }
}

void StatisticsTest::removeSelected()
{
    const QString first = makeImage(QStringLiteral("remove1.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("remove2.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    StatisticsDialog dialog({}, false, 3, false, dir_.path());
    dialog.addFiles({first, second});
    auto* list = child<QListWidget>(dialog, "statisticsFiles");
    QVERIFY(list != nullptr);
    list->setCurrentRow(0);
    child<QPushButton>(dialog, "statisticsRemove")->click();
    QCOMPARE(dialog.paths(), QStringList({second}));
}

void StatisticsTest::addOpenFilesAddsSavedOnly()
{
    const QString saved = makeImage(QStringLiteral("saved.png"), 4, 4, Qt::red);
    QVERIFY(!saved.isEmpty());
    StatisticsDialog dialog({saved}, true, 3, false, dir_.path());
    bool warned = false;
    QTimer closer;
    closer.setInterval(10);
    connect(&closer, &QTimer::timeout, this, [&warned] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* box = qobject_cast<QMessageBox*>(widget)) {
                warned = box->text().contains(QStringLiteral("must be saved"));
                box->accept();
            }
        }
    });
    closer.start();
    dialog.addOpenFiles();
    closer.stop();
    QVERIFY(warned);
    QCOMPARE(dialog.paths(), QStringList({saved}));
}

void StatisticsTest::rememberedChoiceSeedsDialog()
{
    StatisticsDialog dialog({}, false, 9, true, dir_.path());
    QCOMPARE(child<QComboBox>(dialog, "statisticsStackMode")->currentData().toInt(), 9);
    QVERIFY(child<QCheckBox>(dialog, "statisticsAlign")->isChecked());
}

void StatisticsTest::cancelChangesNothing()
{
    QVERIFY(window_->newDocument(QStringLiteral("Untitled"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const quint32 pixel = view->composite_argb(0, 0);
    const int history = view->history_count();
    const int active = window_->activeDocumentIndex();
    const QJsonObject before = pictura::loadSession().statisticsOptions;

    QTimer::singleShot(0, this, [] {
        if (auto* dialog = qobject_cast<StatisticsDialog*>(
                dialogNamed(QStringLiteral("statisticsDialog")))) {
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileScriptsStatistics)));

    QCOMPARE(view->composite_argb(0, 0), pixel);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(window_->activeDocumentIndex(), active);
    QVERIFY(pictura::loadSession().statisticsOptions == before);
}

void StatisticsTest::mergesTwoImagesWithMean()
{
    const QString red = makeImage(QStringLiteral("mean-red.png"), 4, 4, Qt::red);
    const QString blue = makeImage(QStringLiteral("mean-blue.png"), 4, 4, Qt::blue);
    QVERIFY(!red.isEmpty());
    QVERIFY(!blue.isEmpty());
    QVERIFY(runStatistics({red, blue}, 3, false));
    QCOMPARE(window_->documentCount(), 1);
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const auto rows = pictura::layers_api::snapshot(*view, 0, false).rows;
    QCOMPARE(int(rows.size()), 1);
    QVERIFY(rows.first().smartObject);
    const quint32 pixel = view->composite_argb(1, 1);
    QVERIFY2(nearChannel(qRed(pixel), 128), qPrintable(QString::number(qRed(pixel))));
    QVERIFY2(nearChannel(qGreen(pixel), 0), qPrintable(QString::number(qGreen(pixel))));
    QVERIFY2(nearChannel(qBlue(pixel), 128), qPrintable(QString::number(qBlue(pixel))));
    QVERIFY(window_->documentPath(0).isEmpty());
    QVERIFY(window_->isDocumentDirty(0));
}

void StatisticsTest::selectedStackModeIsApplied()
{
    const QString red = makeImage(QStringLiteral("max-red.png"), 4, 4, Qt::red);
    const QString blue = makeImage(QStringLiteral("max-blue.png"), 4, 4, Qt::blue);
    QVERIFY(!red.isEmpty());
    QVERIFY(!blue.isEmpty());
    QVERIFY(runStatistics({red, blue}, 1, false));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const quint32 pixel = view->composite_argb(1, 1);
    QCOMPARE(qRed(pixel), 255);
    QCOMPARE(qGreen(pixel), 0);
    QCOMPARE(qBlue(pixel), 255);
}

void StatisticsTest::unreadableSourceIsSkipped()
{
    const QString first = makeImage(QStringLiteral("partial-a.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("partial-b.png"), 4, 4, Qt::blue);
    const QString corrupt = makeCorrupt(QStringLiteral("corrupt-a.png"));
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    QVERIFY(!corrupt.isEmpty());
    QString warning;
    QVERIFY(runStatistics({first, second, corrupt}, 3, false, &warning));
    QCOMPARE(window_->documentCount(), 1);
    QVERIFY2(warning.contains(QStringLiteral("corrupt-a.png")), qPrintable(warning));
}

void StatisticsTest::fewerThanTwoDecodableCreatesNothing()
{
    const QString first = makeImage(QStringLiteral("one-a.png"), 4, 4, Qt::red);
    const QString corrupt = makeCorrupt(QStringLiteral("corrupt-b.png"));
    QVERIFY(!first.isEmpty());
    QVERIFY(!corrupt.isEmpty());
    QString warning;
    QVERIFY(runStatistics({first, corrupt}, 3, false, &warning));
    QCOMPARE(window_->documentCount(), 0);
    QVERIFY2(warning.contains(QStringLiteral("corrupt-b.png")), qPrintable(warning));
}

void StatisticsTest::acceptedChoiceIsStored()
{
    const QString first = makeImage(QStringLiteral("remember-a.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("remember-b.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    QVERIFY(runStatistics({first, second}, 4, true));
    const pictura::SessionState state = pictura::loadSession();
    QCOMPARE(state.statisticsOptions.value(QStringLiteral("mode")).toInt(), 4);
    QVERIFY(state.statisticsOptions.value(QStringLiteral("align")).toBool());
}

void StatisticsTest::rememberedChoiceSeedsTheNextInvocation()
{
    const QString first = makeImage(QStringLiteral("next-choice-a.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("next-choice-b.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    QVERIFY(runStatistics({first, second}, 1, false));
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }

    bool inspected = false;
    int mode = -1;
    bool align = true;
    QTimer::singleShot(0, this, [&inspected, &mode, &align] {
        auto* dialog = qobject_cast<StatisticsDialog*>(
            dialogNamed(QStringLiteral("statisticsDialog")));
        if (!dialog) {
            return;
        }
        inspected = true;
        mode = child<QComboBox>(*dialog, "statisticsStackMode")->currentData().toInt();
        align = child<QCheckBox>(*dialog, "statisticsAlign")->isChecked();
        dialog->reject();
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileScriptsStatistics)));
    QVERIFY(inspected);
    QCOMPARE(mode, 1);
    QVERIFY(!align);
}

void StatisticsTest::worksWithoutDocument()
{
    QCOMPARE(window_->documentCount(), 0);
    const QString first = makeImage(QStringLiteral("open-a.png"), 4, 4, Qt::red);
    const QString second = makeImage(QStringLiteral("open-b.png"), 4, 4, Qt::blue);
    QVERIFY(!first.isEmpty());
    QVERIFY(!second.isEmpty());
    QVERIFY(runStatistics({first, second}, 3, false));
    QCOMPARE(window_->documentCount(), 1);
}

PICTURA_QT_TEST_MAIN(StatisticsTest)
#include "tst_statistics.moc"
