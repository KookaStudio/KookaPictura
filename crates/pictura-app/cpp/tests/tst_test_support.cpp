#include <QtTest/QtTest>

#include "qt_test_support.h"

namespace {

// A stand-in suite whose test functions the share helpers enumerate.
class SampleSuite : public QObject {
    Q_OBJECT

private slots:
    void init() {}
    void cleanup() {}
    void initTestCase() {}
    void cleanupTestCase() {}
    void initTestCase_data() {}
    void alpha() {}
    void beta() {}
    void gamma() {}
    void delta() {}
    void withData_data() {}
    void withData() {}
    void withParameter(int) {}
    int nonVoid() { return 0; }

public slots:
    void publicSlot() {}
};

int driverProbeCalls = 0;
bool driverProbeNullTerminated = false;
QList<QByteArray> driverProbeArguments;

int driverProbeMain(int argc, char* argv[])
{
    ++driverProbeCalls;
    driverProbeNullTerminated = argv[argc] == nullptr;
    driverProbeArguments.clear();
    for (int i = 0; i < argc; ++i) {
        driverProbeArguments << QByteArray(argv[i]);
    }
    return 23;
}

} // namespace

class TestSupportTest : public QObject {
    Q_OBJECT

private slots:
    void namedFunctionsAreKept();
    void optionValuesAreNotFunctions();
    void perfcounterValueIsNotAFunction();
    void shardAppendsItsShare();
    void emptyShareRunsNothing();
    void testFunctionsExcludeLifecycleAndInvalidSlots();
    void driverDispatchesSelectedSuite();
    void driverRejectsInvalidSelection();
};

void TestSupportTest::namedFunctionsAreKept()
{
    SampleSuite suite;
    const QList<QByteArray> argv = {"exe", "beta"};
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 0, 1), argv);
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 1, 4), argv);
}

void TestSupportTest::optionValuesAreNotFunctions()
{
    SampleSuite suite;
    const QList<QByteArray> argv = {"exe", "-o", "out.xml,junitxml"};
    const QList<QByteArray> expected = {"exe", "-o", "out.xml,junitxml", "alpha", "beta",
                                        "gamma", "delta", "withData"};
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 0, 1), expected);
}

void TestSupportTest::perfcounterValueIsNotAFunction()
{
    SampleSuite suite;
    const QList<QByteArray> argv = {"exe", "-perfcounter", "cycles"};
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 1, 4),
             (QList<QByteArray>{"exe", "-perfcounter", "cycles", "beta"}));
}

void TestSupportTest::shardAppendsItsShare()
{
    SampleSuite suite;
    const QList<QByteArray> argv = {"exe"};
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 1, 4),
             (QList<QByteArray>{"exe", "beta"}));
    QCOMPARE(pictura::test::shardArguments(argv, suite.metaObject(), 0, 4),
             (QList<QByteArray>{"exe", "alpha", "withData"}));
}

void TestSupportTest::emptyShareRunsNothing()
{
    SampleSuite suite;
    const QList<QByteArray> argv = {"exe"};
    // Five functions over eight shares leave shares 5 to 7 empty.
    QVERIFY(pictura::test::shareRunsNothing(argv, suite.metaObject(), 6, 8));
    QVERIFY(!pictura::test::shareRunsNothing(argv, suite.metaObject(), 4, 8));
    QVERIFY(!pictura::test::shareRunsNothing({"exe", "beta"}, suite.metaObject(), 6, 8));
}

void TestSupportTest::testFunctionsExcludeLifecycleAndInvalidSlots()
{
    SampleSuite suite;
    QCOMPARE(pictura::test::testFunctions(suite.metaObject()),
             (QStringList{"alpha", "beta", "gamma", "delta", "withData"}));
}

void TestSupportTest::driverDispatchesSelectedSuite()
{
    static const bool registered = [] {
        pictura::test::registerSuite("C:\\tmp\\tst_driver_probe.cpp", driverProbeMain);
        return true;
    }();
    Q_UNUSED(registered);
    driverProbeCalls = 0;
    driverProbeNullTerminated = false;
    driverProbeArguments.clear();

    char program[] = "driver";
    char suiteFlag[] = "--suite";
    char suiteName[] = "tst_driver_probe";
    char outputFlag[] = "-o";
    char outputName[] = "out.xml,junitxml";
    char function[] = "alpha";
    char* argv[] = {program, suiteFlag, suiteName, outputFlag, outputName, function, nullptr};

    QCOMPARE(pictura::test::runSuiteDriver(6, argv), 23);
    QCOMPARE(driverProbeCalls, 1);
    QVERIFY(driverProbeNullTerminated);
    QCOMPARE(driverProbeArguments,
             (QList<QByteArray>{"tst_driver_probe", "-o", "out.xml,junitxml", "alpha"}));
}

void TestSupportTest::driverRejectsInvalidSelection()
{
    pictura::test::registerSuite("tst_driver_duplicate_probe.cpp", driverProbeMain);
    pictura::test::registerSuite("tst_driver_duplicate_probe.cpp", driverProbeMain);
    driverProbeCalls = 0;

    char program[] = "driver";
    char suiteFlag[] = "--suite";
    char suiteName[] = "tst_driver_probe";
    char emptyName[] = "";
    char cppName[] = "tst_driver_probe.cpp";
    char unknownName[] = "tst_missing_probe";
    char duplicateName[] = "tst_driver_duplicate_probe";
    char function[] = "alpha";

    char* noSelector[] = {program, nullptr};
    char* incomplete[] = {program, suiteFlag, nullptr};
    char* wrongPosition[] = {program, function, suiteFlag, suiteName, nullptr};
    char* empty[] = {program, suiteFlag, emptyName, nullptr};
    char* cppSuffix[] = {program, suiteFlag, cppName, nullptr};
    char* unknown[] = {program, suiteFlag, unknownName, nullptr};
    char* duplicate[] = {program, suiteFlag, duplicateName, nullptr};

    QVERIFY(pictura::test::runSuiteDriver(1, noSelector) != 0);
    QVERIFY(pictura::test::runSuiteDriver(2, incomplete) != 0);
    QVERIFY(pictura::test::runSuiteDriver(4, wrongPosition) != 0);
    QVERIFY(pictura::test::runSuiteDriver(3, empty) != 0);
    QVERIFY(pictura::test::runSuiteDriver(3, cppSuffix) != 0);
    QVERIFY(pictura::test::runSuiteDriver(3, unknown) != 0);
    QVERIFY(pictura::test::runSuiteDriver(3, duplicate) != 0);
    QCOMPARE(driverProbeCalls, 0);
}

PICTURA_QT_TEST_SHARDED_MAIN(TestSupportTest)

#include "tst_test_support.moc"
