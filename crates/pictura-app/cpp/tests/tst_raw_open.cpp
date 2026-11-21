// Camera raw opens through File > Open (#400, `document/raw-open`): signature
// routing, the Pictura Raw dialog before the document, the non-interactive
// develop, the absolute white balance, and the unsupported-camera refusal; and
// the settings stores (#401, `document/raw-settings-storage`); and Save Image
// (#402, `document/raw-save-image`). The fixture DNG
// is written by `raw_fixture.h`. The suite shares one frame, never shown, and
// points XDG_DATA_HOME at a temporary folder so OK never writes to the user's
// Camera Raw database; cleanup() closes its documents, clears the dialog
// driver, the status message and the database, and resets the store.

#include <QtTest/QtTest>

#include "acr/acr_controls.h"
#include "acr/acr_panels.h"
#include "acr/acr_settings.h"
#include "dialogs.h"
#include "frame.h"
#include "import_save_coordinator.h"
#include "pictura_raw_dialog.h"
#include "preferences_dialog.h"
#include "raw_save_options_dialog.h"

#include "api/document_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
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

QComboBox* rawStoreCombo(PicturaMainWindow& window)
{
    window.showPreferences(PreferencesDialog::kFileHandling);
    return window.preferencesDialog()->findChild<QComboBox*>(
        QStringLiteral("preferencesRawSettingsStore"));
}

QByteArray readFile(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

// Whether the process can still create files in `dir` (true for root).
bool writable(const QString& dir)
{
    QFile probe(QDir(dir).filePath(QStringLiteral(".probe")));
    const bool ok = probe.open(QIODevice::WriteOnly);
    probe.remove();
    return ok;
}

// Restores an environment variable when it goes out of scope.
class ScopedEnv {
public:
    ScopedEnv(const char* name, const QByteArray& value)
        : name_(name), had_(qEnvironmentVariableIsSet(name)), prior_(qgetenv(name))
    {
        qputenv(name, value);
    }
    ~ScopedEnv()
    {
        if (had_) {
            qputenv(name_, prior_);
        } else {
            qunsetenv(name_);
        }
    }
    ScopedEnv(const ScopedEnv&) = delete;
    ScopedEnv& operator=(const ScopedEnv&) = delete;

private:
    const char* name_;
    bool had_;
    QByteArray prior_;
};

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

    void settingsStorePreferencePersists();
    void okWritesSidecar();
    void doneWritesDatabaseOnly();
    void cancelAndNonInteractiveWriteNothing();
    void unwritableStoresWarnAndOpen();

    void saveImageOnlyInRawOpen();
    void saveOptionsDefaults();
    void saveImageWritesDng();
    void declinedReplaceWritesNothing();

private:
    std::function<bool(QDialog&)> driver(Seen* seen, bool accept);
    ImportSaveCoordinator* coordinator() const;
    QString databaseDir() const;
    QStringList databaseRecords() const;

    test::ScopedStateHome stateHome_;
    QTemporaryDir dataHome_;
    std::unique_ptr<ScopedEnv> dataEnv_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void RawOpenTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dataHome_.isValid());
    dataEnv_ = std::make_unique<ScopedEnv>("XDG_DATA_HOME", dataHome_.path().toUtf8());
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
    if (ImportSaveCoordinator* importSave = coordinator()) {
        importSave->setRawSettingsStore(0);
    }
    QDir(databaseDir()).removeRecursively();
}

void RawOpenTest::cleanupTestCase()
{
    window_.reset();
    dataEnv_.reset();
}

ImportSaveCoordinator* RawOpenTest::coordinator() const
{
    return window_ ? window_->findChild<ImportSaveCoordinator*>() : nullptr;
}

QString RawOpenTest::databaseDir() const
{
    return dataHome_.filePath(QStringLiteral("kooka-pictura/Camera Raw"));
}

QStringList RawOpenTest::databaseRecords() const
{
    return QDir(databaseDir()).entryList({QStringLiteral("*.xmp")}, QDir::Files);
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

// raw_store_preference
void RawOpenTest::settingsStorePreferencePersists()
{
    test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto window = test::makeMainWindow();
        auto* importSave = window->findChild<ImportSaveCoordinator*>();
        auto* combo = rawStoreCombo(*window);
        QVERIFY(importSave && combo);
        QCOMPARE(importSave->rawSettingsStore(), 0);
        QCOMPARE(combo->currentIndex(), 0);
        combo->setCurrentIndex(1);
        QCOMPARE(importSave->rawSettingsStore(), 1);
    }
    auto window = test::makeMainWindow();
    auto* combo = rawStoreCombo(*window);
    QVERIFY(combo);
    QCOMPARE(window->findChild<ImportSaveCoordinator*>()->rawSettingsStore(), 1);
    QCOMPARE(combo->currentIndex(), 1);
}

// raw_store_ok_writes
void RawOpenTest::okWritesSidecar()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    QVERIFY(!path.isEmpty());
    coordinator()->setRawSettingsStore(1);
    window_->setImportDialogDriverForTest([](QDialog& dialog) {
        auto* exposure = dialog.findChild<AcrSlider*>(QString::fromLatin1(acr::Exposure));
        if (!exposure) {
            return false;
        }
        exposure->setValue(1.0);
        emit exposure->valueChanged();
        return true;
    });
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(window_->documentCount(), 1);
    const QByteArray sidecar = readFile(dir.filePath(QStringLiteral("shot.xmp")));
    QVERIFY2(sidecar.contains("crs:Exposure2012=\"+1.00\""), sidecar.constData());
    QVERIFY2(sidecar.contains("crs:WhiteBalance=\"As Shot\""), sidecar.constData());
    QVERIFY(databaseRecords().isEmpty());
}

// raw_store_done_writes
void RawOpenTest::doneWritesDatabaseOnly()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    int doneButtons = 0;
    window_->setImportDialogDriverForTest([&doneButtons](QDialog& dialog) {
        auto* done = dialog.findChild<QPushButton*>(QStringLiteral("rawDoneButton"));
        if (done) {
            ++doneButtons;
            done->click();
        }
        return false;
    });
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(doneButtons, 1);
    QCOMPARE(window_->documentCount(), 0);
    QCOMPARE(databaseRecords().size(), 1);
    QVERIFY(!QFile::exists(dir.filePath(QStringLiteral("shot.xmp"))));
}

// raw_store_cancel_writes_nothing
void RawOpenTest::cancelAndNonInteractiveWriteNothing()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    coordinator()->setRawSettingsStore(1);
    Seen seen;
    window_->setImportDialogDriverForTest(driver(&seen, false));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(window_->openDocumentAtPathOutcome(path, false),
             PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(seen.rawDialogs, 1);
    QVERIFY(!QFile::exists(dir.filePath(QStringLiteral("shot.xmp"))));
    QVERIFY(databaseRecords().isEmpty());
}

// raw_store_unwritable_warns
void RawOpenTest::unwritableStoresWarnAndOpen()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    const QString blocker = writeFile(dir, QStringLiteral("not-a-folder"), QByteArray("x"));
    QVERIFY(!path.isEmpty() && !blocker.isEmpty());
    const QFileDevice::Permissions normal = QFile::permissions(dir.path());
    QVERIFY(QFile::setPermissions(dir.path(), QFileDevice::ReadOwner | QFileDevice::ExeOwner));
    if (writable(dir.path())) {
        QFile::setPermissions(dir.path(), normal);
        QSKIP("the process can write to a read-only folder");
    }
    ScopedEnv data("XDG_DATA_HOME", blocker.toUtf8());
    coordinator()->setRawSettingsStore(1);
    Seen seen;
    window_->setImportDialogDriverForTest(driver(&seen, true));
    const auto outcome = window_->openDocumentAtPathOutcome(path, true);
    const QString message = window_->statusBar()->currentMessage();
    QFile::setPermissions(dir.path(), normal);
    QCOMPARE(outcome, PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(window_->documentCount(), 1);
    QVERIFY2(message.contains(QStringLiteral("not saved")), qPrintable(message));
    QVERIFY(!QFile::exists(dir.filePath(QStringLiteral("shot.xmp"))));
}

// Drives a raw open's Save Image: `options` sees Save Options and returns
// whether to accept it, `replace` answers the replace question, and the raw
// dialog's status is kept in `status`; the raw dialog is then cancelled.
static std::function<bool(QDialog&)> saveImageDriver(std::function<bool(RawSaveOptionsDialog&)> options,
                                              bool replace, QString* status, int* rawDialogs)
{
    return [=](QDialog& dialog) {
        if (auto* save = qobject_cast<RawSaveOptionsDialog*>(&dialog)) {
            return options(*save);
        }
        if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
            box->button(replace ? QMessageBox::Yes : QMessageBox::No)->click();
            return true;
        }
        if (qobject_cast<PicturaRawDialog*>(&dialog)) {
            ++*rawDialogs;
            auto* button = dialog.findChild<QPushButton*>(QStringLiteral("rawSaveImageButton"));
            if (button) {
                button->click();
            }
            if (auto* label = dialog.findChild<QLabel*>(QStringLiteral("rawSaveStatus"))) {
                *status = label->text();
            }
        }
        return false;
    };
}

// raw_save_button
void RawOpenTest::saveImageOnlyInRawOpen()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    int buttons = 0;
    window_->setImportDialogDriverForTest([&buttons](QDialog& dialog) {
        if (dialog.findChild<QPushButton*>(QStringLiteral("rawSaveImageButton"))) {
            ++buttons;
        }
        return true;
    });
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Opened);
    QCOMPARE(buttons, 1);
    PicturaRawDialog filter(window_->viewAt(0), QString(), QString());
    QVERIFY(!filter.findChild<QPushButton*>(QStringLiteral("rawSaveImageButton")));
}

// raw_save_options_defaults
void RawOpenTest::saveOptionsDefaults()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    QString folder, name, status;
    int compatibility = -1, preview = -1, rawDialogs = 0;
    bool embed = true, linear = true;
    window_->setImportDialogDriverForTest(saveImageDriver(
        [&](RawSaveOptionsDialog& options) {
            folder = options.findChild<QLineEdit*>(QStringLiteral("rawSaveFolder"))->text();
            name = options.findChild<QLineEdit*>(QStringLiteral("rawSaveName"))->text();
            compatibility = options.compatibility();
            preview = options.preview();
            embed = options.embedOriginal();
            linear = options.linear();
            return false;
        },
        false, &status, &rawDialogs));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(rawDialogs, 1);
    QCOMPARE(QDir::fromNativeSeparators(folder), QFileInfo(path).absolutePath());
    // The source itself is shot.dng, so the default takes the next free name.
    QCOMPARE(name, QStringLiteral("shot-1.dng"));
    QCOMPARE(compatibility, 4);
    QCOMPARE(preview, 1);
    QVERIFY(!embed);
    QVERIFY(!linear);
    QVERIFY(status.isEmpty());
}

// raw_save_writes_dng
void RawOpenTest::saveImageWritesDng()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    QString status;
    int rawDialogs = 0;
    window_->setImportDialogDriverForTest(saveImageDriver(
        [](RawSaveOptionsDialog& options) {
            options.findChild<QLineEdit*>(QStringLiteral("rawSaveName"))
                ->setText(QStringLiteral("copy"));
            options.findChild<QCheckBox*>(QStringLiteral("rawSaveLinear"))->setChecked(true);
            return true;
        },
        false, &status, &rawDialogs));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(rawDialogs, 1);
    const QByteArray written = readFile(dir.filePath(QStringLiteral("copy.dng")));
    QVERIFY2(written.startsWith(QByteArray("II*\0", 4)), qPrintable(status));
    QVERIFY2(status.contains(QStringLiteral("copy.dng")), qPrintable(status));
    QCOMPARE(window_->documentCount(), 0);
    QVERIFY(databaseRecords().isEmpty());
}

// raw_save_confirms_replace
void RawOpenTest::declinedReplaceWritesNothing()
{
    QTemporaryDir dir;
    const QString path = writeFile(dir, QStringLiteral("shot.dng"), test::rawDng(16, 12, true));
    const QString taken = writeFile(dir, QStringLiteral("taken.dng"), QByteArray("keep"));
    QString status;
    int rawDialogs = 0;
    window_->setImportDialogDriverForTest(saveImageDriver(
        [](RawSaveOptionsDialog& options) {
            options.findChild<QLineEdit*>(QStringLiteral("rawSaveName"))
                ->setText(QStringLiteral("taken.dng"));
            return true;
        },
        false, &status, &rawDialogs));
    QCOMPARE(window_->openDocumentAtPathOutcome(path, true),
             PicturaMainWindow::OpenOutcome::Cancelled);
    QCOMPARE(rawDialogs, 1);
    QCOMPARE(readFile(taken), QByteArray("keep"));
    QVERIFY(status.isEmpty());
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<RawOpenTest>(argc, argv);
}
#include "tst_raw_open.moc"
