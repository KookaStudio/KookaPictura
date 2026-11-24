// The Save As catalog table and writer routing: every displayed row's kind,
// canonical suffix, and pattern against the engine catalog, the profile policy,
// the route each row takes through the real save leaf, refusal of unknown
// suffixes, and the remembered/replayed DCS kind.

#include <QtTest/QtTest>

#include <QtCore/QByteArray>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QSet>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QWidget>

#include <memory>

#include "api/engine_bridge.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "import_save_coordinator.h"
#include "save_format_options.h"

#include "qt_test_support.h"

namespace {

QSet<QString> patternSuffixes(const QString& pattern)
{
    QSet<QString> suffixes;
    const int open = pattern.indexOf(QLatin1Char('('));
    const int close = pattern.lastIndexOf(QLatin1Char(')'));
    if (open < 0 || close <= open) {
        return suffixes;
    }
    const QStringList parts =
        pattern.mid(open + 1, close - open - 1).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    for (const QString& part : parts) {
        QString suffix = part;
        if (suffix.startsWith(QStringLiteral("*."))) {
            suffix.remove(0, 2);
        }
        suffixes.insert(suffix.toUpper());
    }
    return suffixes;
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

QByteArray expectedSignature(const QString& name)
{
    if (name == QStringLiteral("png")) {
        return QByteArray::fromHex(QByteArrayLiteral("89504e47"));
    }
    if (name == QStringLiteral("jpeg")) {
        return QByteArray::fromHex(QByteArrayLiteral("ffd8"));
    }
    if (name == QStringLiteral("gif")) {
        return QByteArrayLiteral("GIF89a");
    }
    if (name == QStringLiteral("bmp")) {
        return QByteArrayLiteral("BM");
    }
    if (name == QStringLiteral("tiff")) {
        return QByteArrayLiteral("MM");
    }
    if (name == QStringLiteral("psd") || name == QStringLiteral("psb")) {
        return QByteArrayLiteral("8BPS");
    }
    if (name == QStringLiteral("pdf")) {
        return QByteArrayLiteral("%PDF");
    }
    if (name == QStringLiteral("eps") || name == QStringLiteral("dcs")) {
        return QByteArrayLiteral("%!");
    }
    if (name == QStringLiteral("webp")) {
        return QByteArrayLiteral("RIFF");
    }
    if (name == QStringLiteral("cineon")) {
        return QByteArray::fromHex(QByteArrayLiteral("802a5fd7"));
    }
    return {};
}

} // namespace

class SaveFormatCatalogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void everyDisplayedFormatResolvesItsPathAndKind();
    void allFormatsCoversEveryAlias();
    void profilePromptFollowsTheCatalog();
    void eachDisplayedFormatWritesItsRoute();
    void explicitFormatChoosesNativeRoute();
    void saveAsDialogRefusesUnknownSuffix();
    void dcsCopyKeepsDcs();
    void recordedDcsStepKeepsDcs();

private:
    pictura::PictureView* newView(const QString& background);
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void SaveFormatCatalogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

pictura::PictureView* SaveFormatCatalogTest::newView(const QString& background)
{
    if (!window_->newDocument(QStringLiteral("Format"), 32, 32, QStringLiteral("rgb"), 8,
                              background)) {
        return nullptr;
    }
    return window_->activeView();
}

void SaveFormatCatalogTest::everyDisplayedFormatResolvesItsPathAndKind()
{
    const QVector<pictura::SaveFormatRow>& rows = pictura::saveFormatRows();
    QCOMPARE(static_cast<int>(rows.size()), 14);
    for (int i = 0; i < static_cast<int>(rows.size()); ++i) {
        const pictura::SaveFormatRow& row = rows.at(i);
        QCOMPARE(row.id, i);
        QCOMPARE(pictura::saveFormatIdForSourceName(row.name), row.id);
        QCOMPARE(static_cast<int>(pictura::saveFormatKindForFilter(i)),
                 pictura::save_format_code(row.id));
        QCOMPARE(pictura::defaultSuffixForFilter(i), pictura::save_format_canonical(row.id));
        if (row.kind != pictura::SaveFormatKind::None) {
            QCOMPARE(pictura::saveFormatKey(row.kind), row.name);
        } else {
            QVERIFY(pictura::saveFormatKey(row.kind).isEmpty());
        }
        const QString normalized =
            pictura::pathForSaveFilter(dir_.filePath(QStringLiteral("report.jpg")), i, 0);
        QVERIFY2(normalized.endsWith(row.canonicalSuffix, Qt::CaseInsensitive),
                 qPrintable(row.name));
        if (row.name != QStringLiteral("dcs")) {
            QCOMPARE(static_cast<int>(pictura::saveFormatKindForSuffix(row.canonicalSuffix.mid(1))),
                     pictura::save_format_code(row.id));
        } else {
            QCOMPARE(static_cast<int>(pictura::saveFormatKindForSuffix(QStringLiteral("eps"))),
                     static_cast<int>(pictura::SaveFormatKind::Eps));
            QCOMPARE(pictura::saveFormatIdForSourceName(QStringLiteral("dcs")), row.id);
        }
    }
}

void SaveFormatCatalogTest::allFormatsCoversEveryAlias()
{
    const QVector<pictura::SaveFormatRow>& rows = pictura::saveFormatRows();
    const QStringList& filters = pictura::saveFileFilters();
    QCOMPARE(static_cast<int>(filters.size()), static_cast<int>(rows.size()) + 1);
    QSet<QString> expected;
    for (const pictura::SaveFormatRow& row : rows) {
        const QStringList aliases =
            pictura::save_format_aliases(row.id).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        QSet<QString> rowAliases;
        for (const QString& alias : aliases) {
            rowAliases.insert(alias.toUpper());
        }
        if (row.name == QStringLiteral("dcs")) {
            const QSet<QString> epsOnly{QStringLiteral("EPS")};
            QVERIFY(rowAliases.isEmpty());
            QVERIFY(patternSuffixes(row.pattern) == epsOnly);
        } else {
            QVERIFY2(patternSuffixes(row.pattern) == rowAliases, qPrintable(row.name));
        }
        expected.unite(rowAliases);
    }
    QVERIFY(patternSuffixes(filters.last()) == expected);
    QCOMPARE(static_cast<int>(pictura::saveFormatKindForSuffix(QStringLiteral("xyz"))),
             static_cast<int>(pictura::SaveFormatKind::None));
}

void SaveFormatCatalogTest::profilePromptFollowsTheCatalog()
{
    for (const pictura::SaveFormatRow& row : pictura::saveFormatRows()) {
        const bool expected = row.name == QStringLiteral("psd")
            || row.name == QStringLiteral("psb") || row.name == QStringLiteral("jpeg")
            || row.name == QStringLiteral("tiff");
        QCOMPARE(pictura::saveFormatOffersProfile(row.canonicalSuffix.mid(1)), expected);
    }

    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    int prompts = 0;
    window_->setProfileDialogForTest([&prompts](bool, bool&) {
        ++prompts;
        return true;
    });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    QVERIFY(window_->saveActiveAsWithOptions(dir_.filePath(QStringLiteral("profile.psd"))));
    QCOMPARE(prompts, 1);
    QVERIFY(window_->saveActiveAsWithOptions(dir_.filePath(QStringLiteral("profile.png"))));
    QCOMPARE(prompts, 1);
    window_->setProfileDialogForTest(nullptr);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatCatalogTest::eachDisplayedFormatWritesItsRoute()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    auto* coordinator = window_->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    coordinator->setNativeSaveCompatibility(0); // Always
    window_->setWarningDriverForTest([](const QString&, const QString&) { return true; });
    window_->setProfileDialogForTest([](bool, bool&) { return true; });
    window_->setGifDialogForTest(
        [](pictura::PictureView*, pictura::SaveFormatOptions&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            // Start from the CS6 defaults so a remembered setting from an
            // earlier test cannot change the bytes this test checks.
            options = pictura::SaveFormatOptions{};
            options.dcsVersion = 1;
            options.dcsPreview = 0;
            options.epsPreview = 0;
            options.rawHeader = 4;
            return true;
        });
    for (const pictura::SaveFormatRow& row : pictura::saveFormatRows()) {
        const QString target = dir_.filePath(
            QStringLiteral("route-%1-%2%3").arg(row.id).arg(row.name, row.canonicalSuffix));
        QVERIFY2(window_->saveActiveAsWithOptions(target, row.kind), qPrintable(row.name));
        QVERIFY2(QFileInfo::exists(target), qPrintable(row.name));
        const QByteArray bytes = fileBytes(target);
        QVERIFY2(!bytes.isEmpty(), qPrintable(row.name));
        const QFileInfo info(target);
        const QString base = info.path() + QLatin1Char('/') + info.completeBaseName();
        if (row.name == QStringLiteral("psd") || row.name == QStringLiteral("psb")) {
            QVERIFY2(bytes.size() >= 6, qPrintable(row.name));
            QVERIFY2(bytes.left(4) == QByteArrayLiteral("8BPS"), qPrintable(row.name));
            const int version = (static_cast<quint8>(bytes.at(4)) << 8)
                | static_cast<quint8>(bytes.at(5));
            QCOMPARE(version, row.name == QStringLiteral("psd") ? 1 : 2);
        } else if (row.name == QStringLiteral("dcs")) {
            QVERIFY2(bytes.left(2) == QByteArrayLiteral("%!"), "DCS writes an EPS composite");
            QVERIFY2(QFileInfo::exists(base + QStringLiteral(".C")), "the DCS plate set");
        } else if (row.name == QStringLiteral("eps")) {
            QVERIFY2(bytes.left(2) == QByteArrayLiteral("%!"), "EPS header");
            QVERIFY2(!QFileInfo::exists(base + QStringLiteral(".C")),
                     "plain EPS writes no separation plates");
        } else if (row.name == QStringLiteral("raw")) {
            QVERIFY2(bytes.size() > 4, "the RAW header precedes the samples");
            for (int i = 0; i < 4; ++i) {
                QCOMPARE(static_cast<quint8>(bytes.at(i)), 0);
            }
        } else if (row.name == QStringLiteral("tga")) {
            QVERIFY2(bytes.size() > 18, "the TGA header precedes the samples");
            QCOMPARE(static_cast<quint8>(bytes.at(2)), 2);
        } else {
            const QByteArray signature = expectedSignature(row.name);
            if (!signature.isEmpty()) {
                QVERIFY2(bytes.left(signature.size()) == signature, qPrintable(row.name));
            }
        }
    }
    window_->setSaveFormatDialogForTest(nullptr);
    window_->setGifDialogForTest(nullptr);
    window_->setProfileDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

void SaveFormatCatalogTest::explicitFormatChoosesNativeRoute()
{
    QVERIFY(newView(QStringLiteral("white")));
    auto* coordinator = window_->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    coordinator->setNativeSaveCompatibility(0);

    const QString psdPath = dir_.filePath(QStringLiteral("explicit-psd.png"));
    QVERIFY(coordinator->saveActiveAs(
        psdPath, false, std::nullopt,
        pictura::saveFormatIdForSuffix(QStringLiteral("psd"))));
    const QByteArray psdBytes = fileBytes(psdPath);
    QVERIFY(psdBytes.size() >= 6);
    QCOMPARE(psdBytes.left(4), QByteArrayLiteral("8BPS"));
    const int version = (static_cast<quint8>(psdBytes.at(4)) << 8)
        | static_cast<quint8>(psdBytes.at(5));
    QCOMPARE(version, 1);

    const QString pngPath = dir_.filePath(QStringLiteral("explicit-png.psd"));
    QVERIFY(coordinator->saveActiveAs(
        pngPath, false, std::nullopt,
        pictura::saveFormatIdForSuffix(QStringLiteral("png"))));
    QCOMPARE(fileBytes(pngPath).left(4), expectedSignature(QStringLiteral("png")));
    closeActive();
}

void SaveFormatCatalogTest::saveAsDialogRefusesUnknownSuffix()
{
    QVERIFY(newView(QStringLiteral("white")));
    const QString unknown = dir_.filePath(QStringLiteral("dialog-refused.xyz"));
    pictura::setSaveFileNameProviderForTest(
        [unknown](QWidget*, const QString&, const QString&, const QStringList& filters,
                  QString* selected) {
            if (selected) {
                *selected = filters.last();
            }
            return unknown;
        });
    QVERIFY(!window_->saveAsWithDialog());
    QVERIFY(!QFileInfo::exists(unknown));
    pictura::setSaveFileNameProviderForTest({});
    closeActive();
}

void SaveFormatCatalogTest::dcsCopyKeepsDcs()
{
    QVERIFY(newView(QStringLiteral("white")));
    window_->setWarningDriverForTest([](const QString&, const QString&) { return true; });
    window_->setProfileDialogForTest([](bool, bool&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options = pictura::SaveFormatOptions{};
            options.dcsVersion = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(dir_.filePath(QStringLiteral("copy-source.eps")),
                                             pictura::SaveFormatKind::Dcs));
    QVERIFY(QFileInfo::exists(dir_.filePath(QStringLiteral("copy-source.C"))));

    const QString target = dir_.filePath(QStringLiteral("copy-target.eps"));
    pictura::setSaveFileNameProviderForTest(
        [target](QWidget*, const QString&, const QString&, const QStringList& filters,
                 QString* selected) {
            if (selected) {
                for (const QString& filter : filters) {
                    if (filter.startsWith(QStringLiteral("Photoshop DCS"))) {
                        *selected = filter;
                        break;
                    }
                }
            }
            return target;
        });
    QVERIFY(window_->saveCopyWithDialog());
    QVERIFY2(QFileInfo::exists(dir_.filePath(QStringLiteral("copy-target.C"))),
             "the DCS copy writes the plate set");
    pictura::setSaveFileNameProviderForTest({});
    window_->setSaveFormatDialogForTest(nullptr);
    window_->setProfileDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

void SaveFormatCatalogTest::recordedDcsStepKeepsDcs()
{
    QVERIFY(newView(QStringLiteral("white")));
    window_->setWarningDriverForTest([](const QString&, const QString&) { return true; });
    window_->setProfileDialogForTest([](bool, bool&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options = pictura::SaveFormatOptions{};
            options.dcsVersion = 1;
            return true;
        });
    QList<QPair<QString, QJsonObject>> recorded;
    window_->registry()->setRecordingSink(
        [&recorded](const QString& id, const pictura::CommandParams& params) {
            recorded << qMakePair(id, params.values);
        });
    QVERIFY(window_->saveActiveAsWithOptions(dir_.filePath(QStringLiteral("recorded-dcs.eps")),
                                             pictura::SaveFormatKind::Dcs));
    window_->registry()->setRecordingSink(pictura::CommandRecordingSink());
    QCOMPARE(recorded.size(), 1);
    QCOMPARE(recorded.first().first, QStringLiteral("file.saveAs"));
    closeActive();
    QVERIFY(newView(QStringLiteral("white")));

    pictura::CommandParams params;
    params.values = recorded.first().second;
    const QString replay = dir_.filePath(QStringLiteral("replayed-dcs.eps"));
    params.values.insert(QStringLiteral("in"), replay);
    QVERIFY(window_->registry()->executeRecorded(QStringLiteral("file.saveAs"), params));
    QVERIFY2(QFileInfo::exists(dir_.filePath(QStringLiteral("replayed-dcs.C"))),
             "the recorded DCS step replays as DCS");

    window_->setSaveFormatDialogForTest(nullptr);
    window_->setProfileDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

QTEST_MAIN(SaveFormatCatalogTest)
#include "tst_save_format_catalog.moc"
