#include "qt_test_support.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QMetaMethod>
#include <QtCore/QString>
#include <QtWidgets/QCompleter>
#include <QtWidgets/QFontComboBox>

#include "frame.h"

namespace pictura::test {

ScopedDefaultLocale::ScopedDefaultLocale(const QLocale& locale)
{
    QLocale::setDefault(locale);
}

ScopedDefaultLocale::~ScopedDefaultLocale()
{
    QLocale::setDefault(previous_);
}

ScopedStateHome::ScopedStateHome()
    : valid_(dir_.isValid())
    , hadPrior_(qEnvironmentVariableIsSet("XDG_STATE_HOME"))
    , prior_(hadPrior_ ? qgetenv("XDG_STATE_HOME") : QByteArray())
    , hadConfig_(qEnvironmentVariableIsSet("XDG_CONFIG_HOME"))
    , config_(hadConfig_ ? qgetenv("XDG_CONFIG_HOME") : QByteArray())
    , hadSystems_(qEnvironmentVariableIsSet("XDG_CONFIG_DIRS"))
    , systems_(hadSystems_ ? qgetenv("XDG_CONFIG_DIRS") : QByteArray())
{
    if (valid_) {
        qputenv("XDG_STATE_HOME", dir_.path().toUtf8());
        qputenv("XDG_CONFIG_HOME", (dir_.path() + QStringLiteral("/config")).toUtf8());
        qputenv("XDG_CONFIG_DIRS", (dir_.path() + QStringLiteral("/system")).toUtf8());
    }
}

ScopedStateHome::~ScopedStateHome()
{
    if (!valid_) {
        return;
    }
    if (hadPrior_) {
        qputenv("XDG_STATE_HOME", prior_);
    } else {
        qunsetenv("XDG_STATE_HOME");
    }
    if (hadConfig_) {
        qputenv("XDG_CONFIG_HOME", config_);
    } else {
        qunsetenv("XDG_CONFIG_HOME");
    }
    if (hadSystems_) {
        qputenv("XDG_CONFIG_DIRS", systems_);
    } else {
        qunsetenv("XDG_CONFIG_DIRS");
    }
}

std::unique_ptr<PicturaMainWindow> makeMainWindow(QWidget* parent)
{
    return std::make_unique<PicturaMainWindow>(parent, false);
}

bool completionOffers(QFontComboBox* combo, const QString& family)
{
    const QCompleter* completer = combo ? combo->completer() : nullptr;
    const QAbstractItemModel* model = completer ? completer->completionModel() : nullptr;
    if (!model) {
        return false;
    }
    for (int row = 0; row < model->rowCount(); ++row) {
        if (model->index(row, 0).data().toString().compare(family, Qt::CaseInsensitive) == 0) {
            return true;
        }
    }
    return false;
}

void selectFamily(QFontComboBox* combo, const QString& family)
{
    const int index = combo->findText(family, Qt::MatchFixedString);
    QVERIFY2(index >= 0, qPrintable(family));
    combo->setCurrentIndex(index);
    QCoreApplication::processEvents();
    QCOMPARE(combo->currentText(), family);
}

void typeFamily(QFontComboBox* combo, const QString& text)
{
    combo->setFocus();
    QTest::keyClicks(combo, text);
    QCoreApplication::processEvents();
}

QStringList testFunctions(const QMetaObject* metaObject)
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

QStringList shardTestFunctions(const QMetaObject* metaObject, int index, int count)
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

bool namesTestFunctions(const QList<QByteArray>& arguments)
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

QList<QByteArray> shardArguments(const QList<QByteArray>& arguments,
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

bool shareRunsNothing(const QList<QByteArray>& arguments, const QMetaObject* metaObject,
                      int index, int count)
{
    return !namesTestFunctions(arguments)
           && shardTestFunctions(metaObject, index, count).isEmpty();
}

} // namespace pictura::test
