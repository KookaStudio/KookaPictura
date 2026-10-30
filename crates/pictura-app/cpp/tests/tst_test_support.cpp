#include <QtTest/QtTest>

#include "qt_test_support.h"

namespace {

// A stand-in suite whose test functions the share helpers enumerate.
class SampleSuite : public QObject {
    Q_OBJECT

private slots:
    void init() {}
    void alpha() {}
    void beta() {}
    void gamma() {}
    void delta() {}
    void withData_data() {}
    void withData() {}
};

} // namespace

class TestSupportTest : public QObject {
    Q_OBJECT

private slots:
    void namedFunctionsAreKept();
    void optionValuesAreNotFunctions();
    void perfcounterValueIsNotAFunction();
    void shardAppendsItsShare();
    void emptyShareRunsNothing();
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

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<TestSupportTest>(argc, argv);
}

#include "tst_test_support.moc"
