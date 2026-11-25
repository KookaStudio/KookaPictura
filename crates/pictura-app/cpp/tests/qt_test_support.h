#pragma once

#include <memory>

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QLocale>
#include <QtCore/QMetaObject>
#include <QtCore/QScopedPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>
#include <QtWidgets/QApplication>

class QFontComboBox;
class QWidget;

namespace pictura {
class PicturaMainWindow;
}

namespace pictura::test {

class ScopedDefaultLocale {
public:
    explicit ScopedDefaultLocale(const QLocale& locale);
    ~ScopedDefaultLocale();

    ScopedDefaultLocale(const ScopedDefaultLocale&) = delete;
    ScopedDefaultLocale& operator=(const ScopedDefaultLocale&) = delete;

private:
    const QLocale previous_;
};

// Isolate both profile roots for the lifetime of a test window.
// Construct it before the window so it outlives the window's session save.
class ScopedStateHome {
public:
    ScopedStateHome();
    ~ScopedStateHome();

    ScopedStateHome(const ScopedStateHome&) = delete;
    ScopedStateHome& operator=(const ScopedStateHome&) = delete;

    bool isValid() const { return valid_; }

private:
    QTemporaryDir dir_;
    bool valid_;
    bool hadPrior_;
    QByteArray prior_;
    bool hadConfig_;
    QByteArray config_;
    bool hadSystems_;
    QByteArray systems_;
};

std::unique_ptr<PicturaMainWindow> makeMainWindow(QWidget* parent = nullptr);

// Whether `combo`'s current completion set offers `family`, ignoring case.
bool completionOffers(QFontComboBox* combo, const QString& family);

// Pick an installed family by its exact name, as the popup does.
void selectFamily(QFontComboBox* combo, const QString& family);

// Type into a family field as the user does, then let the completion run.
void typeFamily(QFontComboBox* combo, const QString& text);

// The test functions of a suite, in the order QTest finds them.
//
// QTest runs every private slot without parameters, except the lifecycle
// hooks and the data functions of data-driven tests. The list has to match
// QTest's own, or a share would leave a function unrun.
QStringList testFunctions(const QMetaObject* metaObject);

// The share of the test functions one registered entry runs.
//
// The Qt Test layer registers a slow suite in several entries, so the layer's
// wall time is set by the number of suites and the parallel level rather than
// by one slow suite. The share is chosen by the PICTURA_TEST_SHARD
// environment variable ("<index>/<count>"): the functions are distributed
// over the shares in declaration order. A suite registered without the
// variable runs every function.
QStringList shardTestFunctions(const QMetaObject* metaObject, int index, int count);

// Whether the command line already names test functions to run.
//
// Arguments after the program name that do not start with '-' are function
// names, except the values of the QTest options that take one.
bool namesTestFunctions(const QList<QByteArray>& arguments);

// The QTest command line for one share: the given arguments, followed by the
// share's test functions unless the arguments already name the functions to
// run, in which case they are kept as they are.
QList<QByteArray> shardArguments(const QList<QByteArray>& arguments,
                                 const QMetaObject* metaObject, int index, int count);

// Whether this share has nothing to run: the command line names no functions
// and the share is empty. QTest given no function names runs them all, so such
// a share must not reach it; it is a registration with more shares than the
// suite has functions.
bool shareRunsNothing(const QList<QByteArray>& arguments, const QMetaObject* metaObject,
                      int index, int count);

using SuiteMain = int (*)(int, char**);
bool registerSuite(const char* sourceFile, SuiteMain main);
int runSuiteDriver(int argc, char* argv[]);

// Runs the test object, restricted to the share named by PICTURA_TEST_SHARD.
//
// This is QTEST_MAIN with the share applied: the same application, the same
// navigation mode, and the test functions of the share passed to QTest as the
// functions to run. Functions named on the command line run instead.
//
// A template cannot be compiled out of line, so this definition stays here; it
// is the only body the shared header still carries.
template <typename TestObject>
int shardedMain(int argc, char* argv[])
{
    QApplication app(argc, argv);
    app.setAttribute(Qt::AA_Use96Dpi, true);

    QScopedPointer<TestObject> test(new TestObject);

    int index = 0;
    int count = 1;
    const QByteArray shard = qgetenv("PICTURA_TEST_SHARD");
    const int separator = shard.indexOf('/');
    if (separator > 0) {
        index = shard.left(separator).toInt();
        count = shard.mid(separator + 1).toInt();
    }

    QList<QByteArray> commandLine;
    for (int i = 0; i < argc; ++i) {
        commandLine << QByteArray(argv[i]);
    }
    // Failing names the misregistration; exiting quietly would leave no JUnit
    // report, which the unified report counts as a suite that never ran.
    if (shareRunsNothing(commandLine, test->metaObject(), index, count)) {
        qCritical("share %d/%d holds no test functions; lower the suite's shard count",
                  index, count);
        return 1;
    }
    QList<QByteArray> arguments = shardArguments(commandLine, test->metaObject(), index, count);
    QList<char*> pointers;
    for (QByteArray& argument : arguments) {
        pointers << argument.data();
    }
    pointers << nullptr;
    return QTest::qExec(test.data(), pointers.size() - 1, pointers.data());
}

} // namespace pictura::test

#define PICTURA_QT_TEST_JOIN_INNER(a, b) a##b
#define PICTURA_QT_TEST_JOIN(a, b) PICTURA_QT_TEST_JOIN_INNER(a, b)
#define PICTURA_QT_TEST_NAMESPACE(TestClass) \
    namespace PICTURA_QT_TEST_JOIN(picturaQtTest_, TestClass) {
#define PICTURA_QT_TEST_REGISTER(TestClass) \
    [[maybe_unused]] static const bool PICTURA_QT_TEST_JOIN(picturaQtTestRegistered_, TestClass) = \
        ::pictura::test::registerSuite(__FILE__, &main);
#define PICTURA_QT_TEST_MAIN(TestClass) \
    PICTURA_QT_TEST_NAMESPACE(TestClass) \
    QTEST_MAIN(TestClass) \
    PICTURA_QT_TEST_REGISTER(TestClass) \
    }
#define PICTURA_QT_TEST_GUILESS_MAIN(TestClass) \
    PICTURA_QT_TEST_NAMESPACE(TestClass) \
    QTEST_GUILESS_MAIN(TestClass) \
    PICTURA_QT_TEST_REGISTER(TestClass) \
    }
#define PICTURA_QT_TEST_SHARDED_MAIN(TestClass) \
    PICTURA_QT_TEST_NAMESPACE(TestClass) \
    int main(int argc, char* argv[]) \
    { \
        return ::pictura::test::shardedMain<TestClass>(argc, argv); \
    } \
    PICTURA_QT_TEST_REGISTER(TestClass) \
    }
