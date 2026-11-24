#include <QtTest/QtTest>

#include <memory>
#include <tuple>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QProcess>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtGui/QShortcutEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTableWidget>
#include <QtWidgets/QTabWidget>

#include "api/engine_bridge.h"
#include "canvas_ruler.h"
#include "commands.h"
#include "dialogs.h"
#include "file_info_dialog.h"
#include "fill_dialog.h"
#include "frame_update.h"
#include "localization.h"
#include "new_document_dialog.h"
#include "panels/numeric_field.h"
#include "preferences.h"
#include "preferences_dialog.h"
#include "frame.h"
#include "qt_test_support.h"
#include "tools.h"
#include "api/view_bridge.h"

using namespace pictura;

namespace {

class ScopedDataHome {
public:
    ScopedDataHome()
    {
        if (directory_.isValid()) {
            qputenv("XDG_DATA_HOME", directory_.path().toUtf8());
        }
    }

    ~ScopedDataHome()
    {
        if (!directory_.isValid()) {
            return;
        }
        if (previous_.isNull()) {
            qunsetenv("XDG_DATA_HOME");
        } else {
            qputenv("XDG_DATA_HOME", previous_);
        }
    }

    bool isValid() const { return directory_.isValid(); }

private:
    QTemporaryDir directory_;
    const QByteArray previous_ = qgetenv("XDG_DATA_HOME");
};

bool writeFile(const QString& path, const QByteArray& contents)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(contents) == contents.size();
}

QByteArray frenchCatalog()
{
    return R"(<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE TS>
<TS version="2.1" language="fr_FR">
<context><name>PicturaCommands</name>
<message><source>File</source><translation>Fichier</translation></message>
<message><source>New…</source><translation>Nouveau…</translation></message>
<message><source>Edit</source><translation type="unfinished">Modifier</translation></message>
<message><source>Clear Recent File List</source><translation>&lt;b&gt;Effacer&lt;/b&gt;</translation></message>
</context>
<context><name>PicturaFiles</name>
<message><source>Photoshop EPS</source><translation>Format partagé</translation></message>
<message><source>Photoshop DCS</source><translation>Format partagé</translation></message>
</context>
<context><name>PicturaUi</name>
<message><source>Auto-Select</source><translation>Sélection automatique</translation></message>
<message><source>Show Transform Controls</source><translation>&lt;b&gt;Contrôles&lt;/b&gt;</translation></message>
</context>
<context><name>pictura::PreferencesDialog</name>
<message><source>UI Size</source><translation>&lt;b&gt;Taille&lt;/b&gt;</translation></message>
</context></TS>)";
}

QString firstMenu(const PicturaMainWindow& window)
{
    return window.menuBar()->actions().constFirst()->text();
}

void openInterface(PicturaMainWindow& window)
{
    window.registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesInterface));
}

QComboBox* languagePicker(PicturaMainWindow& window)
{
    return window.findChild<QComboBox*>(QStringLiteral("preferencesLanguage"));
}

void selectLanguage(QComboBox& picker, const QString& selection)
{
    const int index = picker.findData(selection);
    if (index >= 0) {
        picker.setCurrentIndex(index);
        emit picker.activated(index);
    }
}

QString languageStatus(const PicturaMainWindow& window)
{
    const auto* label = window.findChild<QLabel*>(QStringLiteral("preferencesLanguageStatus"));
    return label ? label->text() : QString();
}

QAction* actionWithText(QMenu& menu, const QString& text)
{
    for (QAction* action : menu.actions()) {
        if (action->text() == text) {
            return action;
        }
    }
    return nullptr;
}

void setOpenChoice(const QString& path)
{
    setOpenFileNameProviderForTest(
        [path](QWidget*, const QString&, const QString&, const QStringList&, QString*) {
            return path;
        });
}

void importFromPreferences(PicturaMainWindow& window, const QString& path)
{
    setOpenChoice(path);
    const auto resetPicker = qScopeGuard([] { setOpenFileNameProviderForTest({}); });
    if (auto* button = window.findChild<QPushButton*>(QStringLiteral("preferencesImportTranslation"))) {
        button->click();
    }
}

} // namespace

class UiLocalizationTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void bundledCatalogsTranslateUi();
    void panelTabsAndContentsUseLanguage();
    void toolOptionsKeepStableValues();
    void standaloneDialogUsesLanguage();
    void unsupportedSystemLanguageUsesEnglish();
    void partialCatalogPreservesCommandIdentity();
    void explicitSelectionAppliesOnRestart();
    void englishAndSystemCanBeSelected();
    void restartButtonFollowsPendingChoice();
    void dynamicMenusKeepIdentity();
    void languageSaveFailurePreservesChoice();
    void selfTestUsesDeterministicLocale();
    void importsAndActivatesLocalCatalog();
    void failedImportPreservesSelection();
    void unavailableImportUsesSystem();
    void importedMarkupRendersAsPlainText();
    void collidingSaveFiltersKeepDcsIdentity();
    void numericLocaleIsIndependent();
    void sharedFieldCommitsCommaDecimal();
    void sharedFieldAppliesLocaleToKeys();

private:
    std::unique_ptr<ScopedDataHome> dataHome_;
};

void UiLocalizationTest::init()
{
    dataHome_ = std::make_unique<ScopedDataHome>();
    QVERIFY(dataHome_->isValid());
}

void UiLocalizationTest::cleanup()
{
    dataHome_.reset();
}

void UiLocalizationTest::bundledCatalogsTranslateUi()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    for (const auto& [language, file, open, move, width] : {
             std::tuple{QStringLiteral("de-AT"), QStringLiteral("Datei"), QStringLiteral("Öffnen..."),
                        QStringLiteral("Verschieben"), QStringLiteral("Breite:")},
             std::tuple{QStringLiteral("fi-FI"), QStringLiteral("Tiedosto"), QStringLiteral("Avaa..."),
                        QStringLiteral("Siirrä"), QStringLiteral("Leveys:")}}) {
        auto translator = installUiTranslation(QStringLiteral("system"), {language});
        QVERIFY(translator);
        for (const auto& command : command_catalogue()) {
            for (const QString& segment : command.path) {
                QVERIFY2(!translator->translate("PicturaCommands", segment.toUtf8().constData()).isNull(),
                         qPrintable(segment));
            }
            if (!command.label.isEmpty()) {
                QVERIFY2(!translator->translate("PicturaCommands", command.label.toUtf8().constData()).isNull(),
                         qPrintable(command.label));
            }
        }
        for (const auto tool : allToolIds()) {
            const auto& info = toolInfo(tool);
            QVERIFY2(!translator->translate("PicturaTools", info.label).isNull(), info.label);
            if (info.hint && *info.hint) {
                QVERIFY2(!translator->translate("PicturaTools", info.hint).isNull(), info.hint);
            }
        }
        for (const char* source : {
                 "Maximize PSD and PSB File Compatibility",
                 "Include a merged image? Other applications may need it to open this file.",
                 "Include Merged Image", "Omit Merged Image",
                 "The chosen format cannot hold all of the document's features. The flattened composite will be written.",
                 "DCS separations need CMYK; the document will be converted.",
                 "DCS cannot carry named spot inks; the extra channels will be omitted."}) {
            QVERIFY2(!translator->translate("PicturaFiles", source).isNull(), source);
        }
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), file);
        QAction* openAction = window->registry()->action(QString::fromLatin1(command_ids::FileOpen));
        QVERIFY(openAction);
        QCOMPARE(openAction->text(), open);
        bool translatedToolTip = false;
        for (auto* widget : window->findChildren<QWidget*>()) {
            translatedToolTip |= widget->toolTip().startsWith(move);
        }
        QVERIFY(translatedToolTip);

        NewDocumentDialog dialog(QStringLiteral("Untitled"), Qt::white);
        bool translatedWidth = false;
        for (auto* label : dialog.findChildren<QLabel*>()) {
            translatedWidth |= label->text() == width;
        }
        QVERIFY(translatedWidth);

        QString openTitle;
        setOpenFileNameProviderForTest(
            [&openTitle](QWidget*, const QString& title, const QString&, const QStringList&, QString*) {
                openTitle = title;
                return QString();
            });
        const auto resetOpen = qScopeGuard([] { setOpenFileNameProviderForTest({}); });
        window->showOpenDialog();
        QVERIFY(!openTitle.isEmpty() && openTitle != QStringLiteral("Open"));

        QVERIFY(window->newDocument(QStringLiteral("Scratch"), 12, 12,
                                    QStringLiteral("rgb"), 8, QStringLiteral("white")));
        QString saveTitle;
        setSaveFileNameProviderForTest(
            [&saveTitle](QWidget*, const QString& title, const QString&, const QStringList&, QString*) {
                saveTitle = title;
                return QString();
            });
        const auto resetSave = qScopeGuard([] { setSaveFileNameProviderForTest({}); });
        window->saveAsWithDialog();
        QVERIFY(!saveTitle.isEmpty() && saveTitle != QStringLiteral("Save As"));

        QString warning;
        window->setFeatureWarningForTest([] { return true; });
        window->setWarningDriverForTest([&warning](const QString&, const QString& text) {
            warning = text;
            return false;
        });
        setSaveFileNameProviderForTest(
            [](QWidget*, const QString&, const QString&, const QStringList& filters, QString* selected) {
                *selected = filters.at(10);
                return QStringLiteral("/unused-localization-test.eps");
            });
        window->saveAsWithDialog();
        QCOMPARE(warning, language == QStringLiteral("de-AT")
                     ? QStringLiteral("DCS-Separationen benötigen CMYK; das Dokument wird konvertiert.")
                     : QStringLiteral("DCS-erottelut edellyttävät CMYK-tilaa; asiakirja muunnetaan."));

        openInterface(*window);
        auto* preferences = window->findChild<PreferencesDialog*>();
        QVERIFY(preferences);
        QVERIFY(preferences->windowTitle() != QStringLiteral("Preferences"));
    }
}

void UiLocalizationTest::panelTabsAndContentsUseLanguage()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    for (const auto& [language, title, opacity, kind, blendName] : {
             std::tuple{QStringLiteral("de"), QStringLiteral("Ebenen"), QStringLiteral("Deckkraft"),
                        QStringLiteral("Art"), QStringLiteral("Normal")},
             std::tuple{QStringLiteral("fi"), QStringLiteral("Tasot"), QStringLiteral("Peittävyys"),
                        QStringLiteral("Laji"), QStringLiteral("Normaali")}}) {
        auto translator = installUiTranslation(language);
        QVERIFY(translator);
        auto window = test::makeMainWindow();
        auto* layers = window->findChild<QWidget*>(QStringLiteral("layersPanel"));
        QVERIFY(layers);
        bool foundTab = false;
        for (const auto* tabs : window->findChildren<QTabWidget*>()) {
            const int index = tabs->indexOf(layers);
            if (index >= 0) {
                QCOMPARE(tabs->tabText(index), title);
                foundTab = true;
                break;
            }
        }
        QVERIFY(foundTab);
        auto* field = layers->findChild<QWidget*>(QStringLiteral("layersOpacityField"));
        QVERIFY(field);
        QCOMPARE(field->toolTip(), opacity);
        auto* filter = layers->findChild<QComboBox*>(QStringLiteral("layersFilterDimension"));
        auto* blend = layers->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
        QVERIFY(filter && blend);
        QCOMPARE(filter->currentText(), kind);
        QCOMPARE(filter->currentData().toString(), QStringLiteral("kind"));
        QCOMPARE(blend->itemText(0), blendName);
    }
}

void UiLocalizationTest::toolOptionsKeepStableValues()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    for (const auto& [language, autoSelect, group, pointSample, ring, nudge] : {
             std::tuple{QStringLiteral("de"), QStringLiteral("Automatisch auswählen"),
                        QStringLiteral("Gruppe"), QStringLiteral("Punktaufnahme"),
                        QStringLiteral("Aufnahmering anzeigen"), QStringLiteral("Um 1 px verschieben")},
             std::tuple{QStringLiteral("fi"), QStringLiteral("Valitse automaattisesti"),
                        QStringLiteral("Ryhmä"), QStringLiteral("Pistenäyte"),
                        QStringLiteral("Näytä näyterengas"), QStringLiteral("Siirrä 1 px")}}) {
        auto translator = installUiTranslation(language);
        QVERIFY(translator);
        auto window = test::makeMainWindow();
        window->setActiveTool(ToolId::Move);
        auto* automatic = window->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
        auto* target = window->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
        QVERIFY(automatic && target);
        QCOMPARE(automatic->text(), autoSelect);
        QVERIFY(!automatic->toolTip().isEmpty());
        QVERIFY(!automatic->toolTip().startsWith(QStringLiteral("Select the layer")));
        QCOMPARE(target->itemText(0), group);
        QCOMPARE(target->itemData(0).toInt(), 0);
        const auto* info = window->findChild<QLabel*>(QStringLiteral("infoTool"));
        QVERIFY(info && info->text().contains(nudge));

        window->setActiveTool(ToolId::Eyedropper);
        auto* size = window->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
        auto* samplingRing = window->findChild<QCheckBox*>(QStringLiteral("optionsEyedropperRing"));
        QVERIFY(size && samplingRing);
        QCOMPARE(size->itemText(0), pointSample);
        QCOMPARE(size->itemData(0).toInt(), 1);
        QCOMPARE(size->itemData(1).toInt(), 3);
        QCOMPARE(samplingRing->text(), ring);
    }
}

void UiLocalizationTest::standaloneDialogUsesLanguage()
{
    for (const auto& [language, title, pattern, historyLabel] : {
             std::tuple{QStringLiteral("de"), QStringLiteral("Füllen"),
                        QStringLiteral("Muster"), QStringLiteral("Protokoll")},
             std::tuple{QStringLiteral("fi"), QStringLiteral("Täytä"),
                        QStringLiteral("Kuvio"), QStringLiteral("Historia")}}) {
        auto translator = installUiTranslation(language);
        QVERIFY(translator);
        FillDialog dialog(Qt::black, Qt::white);
        QCOMPARE(dialog.windowTitle(), title);
        auto* contents = dialog.findChild<QComboBox*>(QStringLiteral("fillContents"));
        QVERIFY(contents);
        const int index = contents->findText(pattern);
        QVERIFY(index >= 0);
        contents->setCurrentIndex(index);
        QVERIFY(dialog.isPatternFill());
        QCOMPARE(dialog.blendMode(), QStringLiteral("normal"));

        const QString entries = QStringLiteral("Saved\tPicture.psd\nOpened Picture.psd");
        FileInfoDialog metadata({QStringLiteral("Photoshop\tHistory\t") + entries},
                                {QStringLiteral("Description\tdc:title\tTitle\tMy Title")}, {});
        auto* sections = metadata.findChild<QListWidget*>(QStringLiteral("fileInfoCategories"));
        auto* field = metadata.findChild<QLineEdit*>(QStringLiteral("fileInfoField_dc:title"));
        QVERIFY(sections && field);
        QVERIFY(sections->item(0)->text() != QStringLiteral("Description"));
        QCOMPARE(field->text(), QStringLiteral("My Title"));
        auto* rows = metadata.findChild<QTableWidget*>();
        QVERIFY(rows && rows->item(0, 0));
        QCOMPARE(rows->item(0, 0)->text(), historyLabel);
        const auto* history = qobject_cast<QPlainTextEdit*>(rows->cellWidget(0, 1));
        QVERIFY(history && history->isReadOnly());
        QCOMPARE(history->toPlainText(), entries);
    }
}

void UiLocalizationTest::unsupportedSystemLanguageUsesEnglish()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    for (const QStringList& languages : {QStringList{QStringLiteral("fr-FR")},
                                         QStringList{QStringLiteral("en-GB"), QStringLiteral("de-DE")}}) {
        auto translator = installUiTranslation(QStringLiteral("system"), languages);
        QVERIFY(!translator);
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), QStringLiteral("File"));
    }
}

void UiLocalizationTest::partialCatalogPreservesCommandIdentity()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    QTemporaryDir source;
    QVERIFY(source.isValid());
    const QString path = source.filePath(QStringLiteral("French.ts"));
    QVERIFY(writeFile(path, frenchCatalog()));
    QString error;
    const QString choice = importUiTranslation(path, &error);
    QVERIFY2(!choice.isEmpty(), qPrintable(error));
    auto translator = installUiTranslation(choice, {QStringLiteral("de")});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    window->registry()->refresh();
    QCOMPARE(firstMenu(*window), QStringLiteral("Fichier"));
    QCOMPARE(window->menuBar()->actions().at(1)->text(), QStringLiteral("Edit"));
    const QString id = QString::fromLatin1(command_ids::FileNew);
    auto* action = window->registry()->action(id);
    QVERIFY(action);
    QCOMPARE(action->text(), QStringLiteral("Nouveau…"));
    QCOMPARE(action->data().toString(), id);
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Ctrl+N")));
    int dispatched = 0;
    window->registry()->setHandler(id, [&dispatched] { ++dispatched; });
    action->trigger();
    QCOMPARE(dispatched, 1);
    QShortcutEvent shortcut(action->shortcut(), 0, false);
    QApplication::sendEvent(action, &shortcut);
    QCOMPARE(dispatched, 2);
}

void UiLocalizationTest::explicitSelectionAppliesOnRestart()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    {
        auto translator = installUiTranslation(loadPreferences().uiLanguage, {QStringLiteral("de-DE")});
        QVERIFY(translator);
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), QStringLiteral("Datei"));
        openInterface(*window);
        auto* picker = languagePicker(*window);
        QVERIFY(picker && picker->findData(QStringLiteral("fi")) >= 0);
        selectLanguage(*picker, QStringLiteral("fi"));
        QCOMPARE(loadPreferences().uiLanguage, QStringLiteral("fi"));
        QVERIFY(!languageStatus(*window).isEmpty());
        QCOMPARE(firstMenu(*window), QStringLiteral("Datei"));
    }
    auto translator = installUiTranslation(loadPreferences().uiLanguage, {QStringLiteral("de-DE")});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    QCOMPARE(firstMenu(*window), QStringLiteral("Tiedosto"));
}

void UiLocalizationTest::englishAndSystemCanBeSelected()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    const QStringList system{QStringLiteral("fi-FI")};
    const QList<QPair<QString, QString>> choices{
        {QStringLiteral("Tiedosto"), QStringLiteral("en")},
        {QStringLiteral("File"), QStringLiteral("system")},
        {QStringLiteral("Tiedosto"), QString()}};
    for (const auto& [expected, next] : choices) {
        auto translator = installUiTranslation(loadPreferences().uiLanguage, system);
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), expected);
        if (!next.isEmpty()) {
            openInterface(*window);
            auto* picker = languagePicker(*window);
            QVERIFY(picker);
            selectLanguage(*picker, next);
            QCOMPARE(loadPreferences().uiLanguage, next);
        }
    }
}

void UiLocalizationTest::restartButtonFollowsPendingChoice()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    int launches = 0;
    setAppRestartLauncherForTest([&launches](const QString&) {
        ++launches;
        return false;
    });
    const auto resetLauncher = qScopeGuard([] { setAppRestartLauncherForTest({}); });
    {
        auto window = test::makeMainWindow();
        openInterface(*window);
        auto* picker = languagePicker(*window);
        auto* restart = window->findChild<QPushButton*>(QStringLiteral("preferencesRestartLanguage"));
        QVERIFY(picker && restart);
        QVERIFY(restart->isHidden());

        selectLanguage(*picker, QStringLiteral("fi"));
        QCOMPARE(loadPreferences().uiLanguage, QStringLiteral("fi"));
        QVERIFY(!restart->isHidden());
        selectLanguage(*picker, QStringLiteral("system"));
        QVERIFY(restart->isHidden());
        QVERIFY(languageStatus(*window).isEmpty());
        selectLanguage(*picker, QStringLiteral("fi"));
        QVERIFY(!restart->isHidden());

        QVERIFY(window->newDocument(QStringLiteral("Scratch"), 16, 16,
                                    QStringLiteral("rgb"), 8, QStringLiteral("white")));
        auto* view = window->viewAt(0);
        QVERIFY(view);
        view->set_active_layer(QStringLiteral("0"));
        QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                                  QStringLiteral("normal"), false, false));
        QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
        QVERIFY(view->end_paint());
        QVERIFY(window->isDocumentDirty(0));

        const UnsavedChoice cancel = UnsavedChoice::Cancel;
        const UnsavedChoice discard = UnsavedChoice::Discard;
        const auto resetPrompt = qScopeGuard([] { setRequestUnsavedChoice(nullptr); });
        setRequestUnsavedChoice(&cancel);
        restart->click();
        QCOMPARE(launches, 0);
        QVERIFY(!restart->isHidden());

        setRequestUnsavedChoice(&discard);
        restart->click();
        QCOMPARE(launches, 1);
        QCOMPARE(window->documentCount(), 1);
        QVERIFY(!restart->isHidden());
        QCOMPARE(languageStatus(*window), QStringLiteral("Pictura could not be restarted."));
    }
    auto translator = installUiTranslation(loadPreferences().uiLanguage);
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    openInterface(*window);
    auto* restart = window->findChild<QPushButton*>(QStringLiteral("preferencesRestartLanguage"));
    QVERIFY(restart && restart->isHidden());
    QCOMPARE(firstMenu(*window), QStringLiteral("Tiedosto"));
}

void UiLocalizationTest::dynamicMenusKeepIdentity()
{
    test::ScopedStateHome state;
    QTemporaryDir files;
    QVERIFY(state.isValid() && files.isValid());
    QImage image(11, 13, QImage::Format_RGB32);
    image.fill(Qt::green);
    const QString path = files.filePath(QStringLiteral("recent.png"));
    QVERIFY(image.save(path));
    for (const QString& language : {QStringLiteral("de"), QStringLiteral("fi")}) {
        auto translator = installUiTranslation(language, {});
        QVERIFY(translator);
        auto window = test::makeMainWindow();
        window->setRecentFiles({path});
        auto* recent = window->recentMenu();
        QVERIFY(recent);
        QVERIFY(recent->title() != QStringLiteral("Open Recent"));
        emit recent->aboutToShow();
        auto* open = actionWithText(*recent, QStringLiteral("recent.png"));
        QVERIFY(open);
        open->trigger();
        window->waitForOpensForTest();
        QVERIFY(window->activeView());
        QCOMPARE(window->activeView()->document_width(), 11);

        const QString name = QStringLiteral("Localization-") + language;
        QVERIFY(window->createWorkspaceForTest(name));
        window->switchWorkspaceForTest(QStringLiteral("Essentials"));
        auto* workspace = window->registry()->menu(
            {QStringLiteral("Window"), QStringLiteral("Workspace")});
        QVERIFY(workspace);
        QVERIFY(workspace->title() != QStringLiteral("Workspace"));
        emit workspace->aboutToShow();
        auto* activate = actionWithText(*workspace, name);
        QVERIFY(activate);
        activate->trigger();
        QCOMPARE(window->activeWorkspaceForTest(), name);
    }
}

void UiLocalizationTest::languageSaveFailurePreservesChoice()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    auto translator = installUiTranslation(QStringLiteral("de"), {});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    openInterface(*window);
    auto* picker = languagePicker(*window);
    QVERIFY(picker);
    selectLanguage(*picker, QStringLiteral("fi"));
    const QString success = languageStatus(*window);
    QVERIFY(!success.isEmpty());
    const auto backupPath = preferencesFilePath() + QStringLiteral(".bak");
    QVERIFY(QFile::remove(backupPath));
    QVERIFY(QDir().mkpath(backupPath));
    selectLanguage(*picker, QStringLiteral("en"));
    QCOMPARE(picker->currentData().toString(), QStringLiteral("fi"));
    QCOMPARE(loadPreferences().uiLanguage, QStringLiteral("fi"));
    QVERIFY(!languageStatus(*window).isEmpty() && languageStatus(*window) != success);
    QCOMPARE(firstMenu(*window), QStringLiteral("Datei"));

    QVERIFY(QDir().rmdir(backupPath));
    const auto before = loadPreferences();
    auto after = before;
    after.uiScalePercent = 110;
    QVERIFY(savePreferences(before, after));
    QCOMPARE(loadPreferences().uiLanguage, QStringLiteral("fi"));
    QCOMPARE(loadPreferences().uiScalePercent, 110);
}

void UiLocalizationTest::selfTestUsesDeterministicLocale()
{
    QTemporaryDir home;
    QVERIFY(home.isValid());
    auto environment = QProcessEnvironment::systemEnvironment();
    environment.remove(QStringLiteral("LC_ALL"));
    environment.insert(QStringLiteral("LANG"), QStringLiteral("de_DE.UTF-8"));
    environment.insert(QStringLiteral("LANGUAGE"), QStringLiteral("de"));
    environment.insert(QStringLiteral("LC_MESSAGES"), QStringLiteral("de_DE.UTF-8"));
    environment.insert(QStringLiteral("LC_NUMERIC"), QStringLiteral("fi_FI.UTF-8"));
    environment.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("offscreen"));
    for (const auto* key : {"XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME"}) {
        environment.insert(QString::fromLatin1(key), home.path());
    }
    QProcess process;
    process.setProcessEnvironment(environment);
    process.setProcessChannelMode(QProcess::MergedChannels);
    process.start(QStringLiteral(PICTURA_TEST_APP),
                  {QStringLiteral("--headless"), QStringLiteral("--self-test")});
    QVERIFY(process.waitForStarted());
    QVERIFY(process.waitForFinished(180000));
    const auto output = process.readAll();
    QVERIFY2(process.exitStatus() == QProcess::NormalExit && process.exitCode() == 0,
             output.constData());
    QVERIFY(output.contains("pictura self-test: SUMMARY"));
}

void UiLocalizationTest::importsAndActivatesLocalCatalog()
{
    test::ScopedStateHome state;
    QTemporaryDir home;
    QVERIFY(state.isValid() && home.isValid());
    const QString source = home.filePath(QStringLiteral("French.ts"));
    QVERIFY(writeFile(source, frenchCatalog()));
    {
        auto translator = installUiTranslation(QStringLiteral("system"), {QStringLiteral("de")});
        auto window = test::makeMainWindow();
        openInterface(*window);
        auto* picker = languagePicker(*window);
        QVERIFY(picker);
        const int before = picker->count();
        importFromPreferences(*window, QString());
        QCOMPARE(picker->count(), before);
        importFromPreferences(*window, source);
        QCOMPARE(picker->count(), before + 1);
        QCOMPARE(loadPreferences().uiLanguage, QStringLiteral("system"));
        QVERIFY(QFile::remove(source));
        const QString imported = picker->itemData(before).toString();
        QVERIFY(!imported.isEmpty());
        selectLanguage(*picker, imported);
        QCOMPARE(loadPreferences().uiLanguage, imported);
        QCOMPARE(firstMenu(*window), QStringLiteral("Datei"));
    }
    auto translator = installUiTranslation(loadPreferences().uiLanguage, {QStringLiteral("de")});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    QCOMPARE(firstMenu(*window), QStringLiteral("Fichier"));
    QCOMPARE(window->menuBar()->actions().at(1)->text(), QStringLiteral("Edit"));
    window->setActiveTool(ToolId::Move);
    auto* automatic = window->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
    auto* transform = window->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(automatic && transform);
    QCOMPARE(automatic->text(), QStringLiteral("Sélection automatique"));
    QCOMPARE(transform->text(), QStringLiteral("Show Transform Controls"));
}

void UiLocalizationTest::failedImportPreservesSelection()
{
    test::ScopedStateHome state;
    QTemporaryDir home;
    QVERIFY(state.isValid() && home.isValid());
    auto window = test::makeMainWindow();
    openInterface(*window);
    auto* picker = languagePicker(*window);
    QVERIFY(picker);
    const QString source = home.filePath(QStringLiteral("translation.ts"));
    QVERIFY(writeFile(source, frenchCatalog()));
    importFromPreferences(*window, source);
    selectLanguage(*picker, picker->itemData(picker->count() - 1).toString());
    const auto selected = picker->currentData();
    const int count = picker->count();
    auto* preferences = window->findChild<PreferencesDialog*>();
    QVERIFY(preferences);

    auto external = frenchCatalog();
    external.replace("<!DOCTYPE TS>", "<!DOCTYPE TS SYSTEM \"extra.dtd\">");
    const QList<QByteArray> badCatalogs{
        QByteArrayLiteral("<TS>"), QByteArrayLiteral("<unrelated/>"), external,
        QByteArray(16 * 1024 * 1024, ' ')};
    for (const auto& bad : badCatalogs) {
        QVERIFY(writeFile(source, bad));
        preferences->showLanguageStatus(QString());
        importFromPreferences(*window, source);
        QCOMPARE(languageStatus(*window), QStringLiteral("The file is not a compatible Qt Linguist translation."));
        QCOMPARE(picker->count(), count);
        QCOMPARE(picker->currentData(), selected);
        QCOMPARE(loadPreferences().uiLanguage, selected.toString());
    }
    QVERIFY(QFile::remove(source));
    preferences->showLanguageStatus(QString());
    importFromPreferences(*window, source);
    QCOMPARE(languageStatus(*window), QStringLiteral("The file is not a compatible Qt Linguist translation."));
    QCOMPARE(picker->currentData(), selected);
    QCOMPARE(picker->count(), count);
    QCOMPARE(loadPreferences().uiLanguage, selected.toString());
}

void UiLocalizationTest::unavailableImportUsesSystem()
{
    test::ScopedStateHome state;
    QTemporaryDir home;
    QVERIFY(state.isValid() && home.isValid());
    const QString source = home.filePath(QStringLiteral("French.ts"));
    QVERIFY(writeFile(source, frenchCatalog()));
    const auto before = loadPreferences();
    auto after = before;
    after.uiLanguage = importUiTranslation(source, nullptr);
    QVERIFY(!after.uiLanguage.isEmpty());
    QVERIFY(savePreferences(before, after));
    QVERIFY(QDir(uiTranslationsDirectory()).removeRecursively());
    {
        auto translator = installUiTranslation(loadPreferences().uiLanguage, {QStringLiteral("de")});
        QVERIFY(translator);
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), QStringLiteral("Datei"));
    }
    auto translator = installUiTranslation(loadPreferences().uiLanguage, {QStringLiteral("es")});
    QVERIFY(!translator);
    auto window = test::makeMainWindow();
    QCOMPARE(firstMenu(*window), QStringLiteral("File"));
}

void UiLocalizationTest::importedMarkupRendersAsPlainText()
{
    test::ScopedStateHome state;
    QTemporaryDir home;
    QVERIFY(state.isValid() && home.isValid());
    const QString source = home.filePath(QStringLiteral("French.ts"));
    QVERIFY(writeFile(source, frenchCatalog()));
    const QString selection = importUiTranslation(source, nullptr);
    QVERIFY(!selection.isEmpty());
    auto translator = installUiTranslation(selection, {});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    const QString recentFile = home.filePath(QStringLiteral("recent.png"));
    QVERIFY(writeFile(recentFile, QByteArrayLiteral("fixture")));
    window->setRecentFiles({recentFile});
    auto* recent = window->recentMenu();
    QVERIFY(recent);
    emit recent->aboutToShow();
    auto* clear = actionWithText(*recent, QStringLiteral("<b>Effacer</b>"));
    QVERIFY(clear);
    QCOMPARE(clear->toolTip(), QStringLiteral("<qt>&lt;b&gt;Effacer&lt;/b&gt;</qt>"));
    openInterface(*window);
    QLabel* translated = nullptr;
    for (auto* label : window->findChildren<QLabel*>()) {
        if (label->text() == QStringLiteral("<b>Taille</b>")) {
            translated = label;
            break;
        }
    }
    QVERIFY(translated);
    QCOMPARE(translated->textFormat(), Qt::PlainText);
}

void UiLocalizationTest::collidingSaveFiltersKeepDcsIdentity()
{
    test::ScopedStateHome state;
    QTemporaryDir home;
    QVERIFY(state.isValid() && home.isValid());
    const QString source = home.filePath(QStringLiteral("French.ts"));
    QVERIFY(writeFile(source, frenchCatalog()));
    const QString selection = importUiTranslation(source, nullptr);
    QVERIFY(!selection.isEmpty());
    auto translator = installUiTranslation(selection, {});
    QVERIFY(translator);
    auto window = test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Scratch"), 12, 12,
                                QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QStringList filters;
    QString warning;
    window->setFeatureWarningForTest([] { return true; });
    window->setWarningDriverForTest([&warning](const QString&, const QString& text) {
        warning = text;
        return false;
    });
    setSaveFileNameProviderForTest(
        [&filters, &home](QWidget*, const QString&, const QString&, const QStringList& choices, QString* selected) {
            filters = choices;
            *selected = choices.at(10);
            return home.filePath(QStringLiteral("unused.eps"));
        });
    const auto resetSave = qScopeGuard([] { setSaveFileNameProviderForTest({}); });
    QVERIFY(!window->saveAsWithDialog());
    QVERIFY(filters.at(9) != filters.at(10));
    QCOMPARE(warning, QStringLiteral("DCS separations need CMYK; the document will be converted."));
    QVERIFY(!QFile::exists(home.filePath(QStringLiteral("unused.eps"))));
}

void UiLocalizationTest::numericLocaleIsIndependent()
{
    test::ScopedStateHome state;
    QVERIFY(state.isValid());
    for (const auto language : {QLocale::German, QLocale::Finnish}) {
        const test::ScopedDefaultLocale numbers{QLocale(language)};
        auto translator = installUiTranslation(QStringLiteral("en"), {QStringLiteral("de")});
        QVERIFY(!translator);
        auto window = test::makeMainWindow();
        QCOMPARE(firstMenu(*window), QStringLiteral("File"));
        NumericFieldConfig config;
        config.decimals = 1;
        NumericField field(QString(), config);
        field.setValue(1.5);
        QCOMPARE(field.findChild<QLineEdit*>()->text(), QStringLiteral("1,5"));
        QCOMPARE(formatRulerLength(36.0, RulerUnit::Inches, 72.0, false, 0.0),
                 QStringLiteral("0,500 in"));
    }
}

void UiLocalizationTest::sharedFieldCommitsCommaDecimal()
{
    const test::ScopedDefaultLocale numbers{QLocale(QLocale::German)};
    NumericFieldConfig config;
    config.allowFractional = true;
    config.suffix = QStringLiteral("px");
    NumericField field(QString(), config);
    auto* edit = field.findChild<QLineEdit*>();
    auto* suffix = field.findChild<QLabel*>(QStringLiteral("numericSuffix"));
    QVERIFY(edit && suffix);
    QSignalSpy commits(&field, &NumericField::valueCommitted);
    edit->clear();
    QTest::keyClicks(edit, QStringLiteral("1,5"));
    QTest::keyClick(edit, Qt::Key_Return);
    QCOMPARE(commits.count(), 1);
    QCOMPARE(commits.constFirst().constFirst().toDouble(), 1.5);
    QCOMPARE(edit->text() + suffix->text(), QStringLiteral("1,5px"));
}

void UiLocalizationTest::sharedFieldAppliesLocaleToKeys()
{
    const test::ScopedDefaultLocale numbers{QLocale(QLocale::Finnish)};
    NumericFieldConfig config;
    config.maximum = 10.0;
    config.step = 0.5;
    config.decimals = 1;
    NumericField field(QString(), config);
    auto* edit = field.findChild<QLineEdit*>();
    QVERIFY(edit);
    QSignalSpy commits(&field, &NumericField::valueCommitted);
    edit->setText(QStringLiteral("1,5"));
    QTest::keyClick(edit, Qt::Key_Up);
    QCOMPARE(field.value(), 2.0);
    edit->setText(QStringLiteral("15,5"));
    QTest::keyClick(edit, Qt::Key_Return);
    QCOMPARE(field.value(), 10.0);
    for (const auto& invalid : {QStringLiteral("1\u00a0234"), QStringLiteral("bogus")}) {
        edit->setText(invalid);
        QTest::keyClick(edit, Qt::Key_Return);
        QCOMPARE(edit->text(), QStringLiteral("10,0"));
    }
    QCOMPARE(commits.count(), 2);
}

QTEST_MAIN(UiLocalizationTest)
#include "tst_ui_localization.moc"
