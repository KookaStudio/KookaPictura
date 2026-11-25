#include <QtTest/QtTest>

#include <QtCore/QByteArray>

#include "selftest_report.h"
#include "selftest_checks.h"
#include "selftest_fixture.h"
#include "selftest_run.h"
#include "qt_test_support.h"

#include <QtCore/QFileInfo>

#include <cstdio>
#include <cstring>
#include <set>
#include <string>

namespace {

QByteArray drain(std::FILE* sink)
{
    std::fflush(sink);
    std::rewind(sink);
    QByteArray out;
    char buffer[4096];
    size_t read = 0;
    while ((read = std::fread(buffer, 1, sizeof buffer, sink)) > 0) {
        out.append(buffer, int(read));
    }
    return out;
}

struct SinkReport {
    std::FILE* file = std::tmpfile();
    pictura::SelfTestReport report;

    SinkReport() { report.setSink(file); }
    ~SinkReport()
    {
        if (file) {
            std::fclose(file);
        }
    }
    bool ok() const { return file != nullptr; }
    QByteArray text() { return drain(file); }
};

} // namespace

class SelfTestReportTest : public QObject {
    Q_OBJECT

private slots:
    void passTokenCarriesSuiteAndName();
    void skipTokenCarriesReason();
    void failTokenCarriesCode();
    void summaryAggregatesTokens();
    void summaryEmittedOnEarlyReturn();
    void failTokenNamesItsOwnCheck();
    void suiteTransitionEmitsSuiteTokens();
    void failRunEmitsOneSummary();
    void registryAndSuiteMetadataAreConsistent();
    void sharedRunFixtureCleansDocumentsAndRestoresTool();
    void documentFixtureRemovesSeedAndClosesDocument();
};

void SelfTestReportTest::passTokenCarriesSuiteAndName()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("x");
    sink.report.pass("x=1");
    const QByteArray text = sink.text();
    const int suite = text.indexOf("pictura self-test: SUITE core x");
    const int pass = text.indexOf("pictura self-test: PASS core x x=1");
    QVERIFY2(suite >= 0, text.constData());
    QVERIFY2(pass > suite, text.constData());
    QCOMPARE(sink.report.passedCount(), 1);
}

void SelfTestReportTest::skipTokenCarriesReason()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("y");
    sink.report.skip("no adapter");
    const QByteArray text = sink.text();
    QVERIFY2(text.contains("pictura self-test: SKIP core y no adapter"), text.constData());
    QCOMPARE(sink.report.skippedCount(), 1);
}

void SelfTestReportTest::failTokenCarriesCode()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("z");
    QCOMPARE(sink.report.fail(42, "boom"), 42);
    const QByteArray text = sink.text();
    QVERIFY2(text.contains("pictura self-test: FAIL core z 42 boom"), text.constData());
    QCOMPARE(sink.report.failedCount(), 1);
}

void SelfTestReportTest::summaryAggregatesTokens()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("p");
    sink.report.pass("ok");
    sink.report.begin("s");
    sink.report.skip("not run");
    sink.report.begin("f");
    sink.report.fail(7, "bad");
    const int rc = sink.report.finish();
    const QByteArray text = sink.text();
    QVERIFY2(text.contains("pictura self-test: SUMMARY passed=1 failed=1 skipped=1"),
             text.constData());
    QCOMPARE(rc, 1);
    QCOMPARE(text.count("SUMMARY"), 1);
}

void SelfTestReportTest::summaryEmittedOnEarlyReturn()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    const auto delegatedFailure = [&sink]() -> int {
        sink.report.begin("early");
        return sink.report.fail(9, "boom");
    };
    const auto run = [&]() -> int {
        pictura::SelfTestReport::RunGuard guard(sink.report);
        return delegatedFailure();
    };
    QCOMPARE(run(), 9);
    const QByteArray text = sink.text();
    QCOMPARE(text.count("pictura self-test: SUMMARY"), 1);
    QVERIFY2(text.contains("SUMMARY passed=0 failed=1 skipped=0"), text.constData());
}

void SelfTestReportTest::failTokenNamesItsOwnCheck()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("previous");
    sink.report.pass("ok");
    sink.report.begin("own");
    sink.report.fail(9, "late");
    const QByteArray text = sink.text();
    QVERIFY2(text.contains("FAIL core own 9 late"), text.constData());
    QVERIFY2(!text.contains("FAIL core previous"), text.constData());
}

void SelfTestReportTest::suiteTransitionEmitsSuiteTokens()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    sink.report.begin("alpha_check", "alpha");
    sink.report.pass("ok");
    sink.report.begin("beta_check", "beta");
    sink.report.pass("ok");
    const QByteArray text = sink.text();
    const int suiteAlpha = text.indexOf("pictura self-test: SUITE alpha alpha_check");
    const int passAlpha = text.indexOf("pictura self-test: PASS alpha alpha_check ok");
    const int suiteBeta = text.indexOf("pictura self-test: SUITE beta beta_check");
    const int passBeta = text.indexOf("pictura self-test: PASS beta beta_check ok");
    QVERIFY(suiteAlpha >= 0);
    QVERIFY(passAlpha > suiteAlpha);
    QVERIFY(suiteBeta > passAlpha);
    QVERIFY(passBeta > suiteBeta);
    QCOMPARE(text.count("pictura self-test: SUITE"), 2);
}

void SelfTestReportTest::failRunEmitsOneSummary()
{
    SinkReport sink;
    if (!sink.ok()) {
        QSKIP("std::tmpfile() could not create the sink");
    }
    QCOMPARE(sink.report.failRun("profile_open", pictura::selftest_codes::kCode555, "boom"), 555);
    const QByteArray text = sink.text();
    QVERIFY2(text.contains("FAIL core profile_open 555 boom"), text.constData());
    QCOMPARE(text.count("pictura self-test: SUMMARY"), 1);
    QVERIFY2(text.contains("SUMMARY passed=0 failed=1 skipped=0"), text.constData());
}

void SelfTestReportTest::registryAndSuiteMetadataAreConsistent()
{
    std::set<int> values;
    std::set<std::string> names;
    for (const auto& code : pictura::selftest_codes::kCodes) {
        QVERIFY(code.value > 0);
        QVERIFY(values.insert(code.value).second);
        QVERIFY(code.checks && *code.checks);
        for (int retired : pictura::selftest_codes::kRetiredCodes) {
            QVERIFY(code.value != retired);
        }
    }
    for (const auto& check : pictura::selftest::kChecks) {
        QVERIFY(check.name && *check.name);
        QVERIFY(check.suite && *check.suite);
        QVERIFY(names.insert(check.name).second);
    }
    for (const auto& code : pictura::selftest_codes::kCodes) {
        const QByteArray checks(code.checks);
        for (const QByteArray& name : checks.split('|')) {
            QVERIFY2(names.count(name.constData()) == 1, name.constData());
        }
    }
    QCOMPARE(pictura::selftest_codes::kCode002, 2);
    QCOMPARE(pictura::selftest_codes::kCode555, 555);
    QCOMPARE(std::strcmp(pictura::selftest::suiteForCheck("platform_headless"), "core"), 0);
    QCOMPARE(std::strcmp(pictura::selftest::suiteForCheck("profile_open"), "core"), 0);
}

void SelfTestReportTest::sharedRunFixtureCleansDocumentsAndRestoresTool()
{
    pictura::test::ScopedStateHome stateHome;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(stateHome.isValid());
    QVERIFY(window->newDocument(QStringLiteral("Baseline"), 8, 8, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    window->setActiveTool(pictura::ToolId::Brush);
    const int baseline = window->documentCount();
    SinkReport sink;
    QVERIFY(sink.ok());
    {
        pictura::selftest::SelfTestRun run(*window, sink.report);
        QVERIFY(window->newDocument(QStringLiteral("Scoped"), 8, 8, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        QCOMPARE(window->documentCount(), baseline + 1);
        window->setActiveTool(pictura::ToolId::Marquee);
    }
    QCOMPARE(window->documentCount(), baseline);
    QCOMPARE(window->activeTool(), pictura::ToolId::Brush);
    QCOMPARE(sink.text().count("pictura self-test: SUMMARY"), 1);
}

void SelfTestReportTest::documentFixtureRemovesSeedAndClosesDocument()
{
    pictura::test::ScopedStateHome stateHome;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(stateHome.isValid());
    const int baseline = window->documentCount();
    QImage seed(8, 8, QImage::Format_RGB32);
    seed.fill(Qt::red);
    const QString name = QStringLiteral("selftest_fixture_%1").arg(QCoreApplication::applicationPid());
    QString path;
    {
        pictura::selftest_fixture::DocumentFixture fixture(*window, seed, name);
        QVERIFY(fixture.ok());
        path = fixture.path;
        QVERIFY(QFileInfo::exists(path));
        QCOMPARE(window->documentCount(), baseline + 1);
    }
    QVERIFY(!QFileInfo::exists(path));
    QCOMPARE(window->documentCount(), baseline);
}

PICTURA_QT_TEST_MAIN(SelfTestReportTest)
#include "tst_selftest_report.moc"
