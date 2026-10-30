#pragma once

#include <memory>

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QMetaMethod>
#include <QtCore/QMetaObject>
#include <QtCore/QScopedPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>
#include <QtWidgets/QApplication>

#include "frame.h"

namespace pictura::test {

// Redirects XDG_STATE_HOME to a throwaway dir for the guard's lifetime so a
// PicturaMainWindow never reads or writes the developer's real session state.
// Construct it before the window so it outlives the window's session save.
class ScopedStateHome {
public:
    ScopedStateHome()
        : valid_(dir_.isValid())
        , hadPrior_(qEnvironmentVariableIsSet("XDG_STATE_HOME"))
        , prior_(hadPrior_ ? qgetenv("XDG_STATE_HOME") : QByteArray())
    {
        if (valid_) {
            qputenv("XDG_STATE_HOME", dir_.path().toUtf8());
        }
    }

    ~ScopedStateHome()
    {
        if (!valid_) {
            return;
        }
        if (hadPrior_) {
            qputenv("XDG_STATE_HOME", prior_);
        } else {
            qunsetenv("XDG_STATE_HOME");
        }
    }

    ScopedStateHome(const ScopedStateHome&) = delete;
    ScopedStateHome& operator=(const ScopedStateHome&) = delete;

    bool isValid() const { return valid_; }

private:
    QTemporaryDir dir_;
    bool valid_;
    bool hadPrior_;
    QByteArray prior_;
};

inline std::unique_ptr<PicturaMainWindow> makeMainWindow(QWidget* parent = nullptr)
{
    return std::make_unique<PicturaMainWindow>(parent);
}

// The test functions of a suite, in the order QTest finds them.
//
// QTest runs every private slot without parameters, except the lifecycle
// hooks and the data functions of data-driven tests. The list has to match
// QTest's own, or a share would leave a function unrun.
inline QStringList testFunctions(const QMetaObject* metaObject)
{
    QStringList functions;
    for (int i = QObject::staticMetaObject.methodCount(); i < metaObject->methodCount(); ++i) {
        const QMetaMethod method = metaObject->method(i);
        if (method.methodType() != QMetaMethod::Slot) {
            continue;
        }
        if (method.parameterCount() != 0) {
            continue;
        }
        if (method.access() != QMetaMethod::Private) {
            continue;
        }
        const QByteArray signature = method.methodSignature();
        if (signature == "init()" || signature == "cleanup()") {
            continue;
        }
        if (signature == "initTestCase()" || signature == "cleanupTestCase()") {
            continue;
        }
        if (signature.endsWith("_data()")) {
            continue;
        }
        functions << QString::fromLatin1(signature.left(signature.indexOf('(')));
    }
    return functions;
}

// The share of the test functions one registered entry runs.
//
// The Qt Test layer registers a slow suite in several entries, so the layer's
// wall time is set by the number of suites and the parallel level rather than
// by one slow suite. The share is chosen by the PICTURA_TEST_SHARD
// environment variable ("<index>/<count>"): the functions are distributed
// over the shares in declaration order. A suite registered without the
// variable runs every function.
inline QStringList shardTestFunctions(const QMetaObject* metaObject, int index, int count)
{
    const QStringList functions = testFunctions(metaObject);
    if (count <= 1) {
        return functions;
    }
    QStringList selected;
    for (int i = 0; i < functions.size(); ++i) {
        if (i % count == index) {
            selected << functions.at(i);
        }
    }
    return selected;
}

// Whether the command line already names test functions to run.
//
// Arguments after the program name that do not start with '-' are function
// names, except the values of the QTest options that take one.
inline bool namesTestFunctions(const QList<QByteArray>& arguments)
{
    static const QList<QByteArray> optionsWithValue = {
        "-o", "-maxwarnings", "-eventdelay", "-keydelay", "-mousedelay", "-seed",
        "-repeat", "-iterations", "-median", "-minimumvalue", "-minimumtotal", "-perfcounter",
        "-platform",
    };
    for (int i = 1; i < arguments.size(); ++i) {
        const QByteArray& argument = arguments.at(i);
        if (optionsWithValue.contains(argument)) {
            ++i;
        } else if (!argument.startsWith('-')) {
            return true;
        }
    }
    return false;
}

// The QTest command line for one share: the given arguments, followed by the
// share's test functions unless the arguments already name the functions to
// run, in which case they are kept as they are.
inline QList<QByteArray> shardArguments(const QList<QByteArray>& arguments,
                                        const QMetaObject* metaObject, int index, int count)
{
    if (namesTestFunctions(arguments)) {
        return arguments;
    }
    QList<QByteArray> withShare = arguments;
    for (const QString& function : shardTestFunctions(metaObject, index, count)) {
        withShare << function.toLocal8Bit();
    }
    return withShare;
}

// Whether this share has nothing to run: the command line names no functions
// and the share is empty. QTest given no function names runs them all, so such
// a share must not reach it; it is a registration with more shares than the
// suite has functions.
inline bool shareRunsNothing(const QList<QByteArray>& arguments, const QMetaObject* metaObject,
                             int index, int count)
{
    return !namesTestFunctions(arguments)
           && shardTestFunctions(metaObject, index, count).isEmpty();
}

// Runs the test object, restricted to the share named by PICTURA_TEST_SHARD.
//
// This is QTEST_MAIN with the share applied: the same application, the same
// navigation mode, and the test functions of the share passed to QTest as the
// functions to run. Functions named on the command line run instead.
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
    return QTest::qExec(test.data(), pointers.size(), pointers.data());
}

} // namespace pictura::test
