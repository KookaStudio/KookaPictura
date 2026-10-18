#include "control_server.h"

#include <QtCore/QEvent>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonValue>
#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QWidget>

#include <cmath>
#include <functional>

#include "frame.h"
#include "image_view.h"

namespace pictura {

namespace {

bool numberParam(const QJsonObject& params, const QString& key, double& out)
{
    const QJsonValue value = params.value(key);
    if (!value.isDouble()) {
        return false;
    }
    out = value.toDouble();
    return true;
}

bool intParam(const QJsonObject& params, const QString& key, int& out)
{
    double value = 0.0;
    if (!numberParam(params, key, value) || value != std::floor(value)) {
        return false;
    }
    out = int(value);
    return true;
}

bool parseButton(const QJsonObject& params, Qt::MouseButton& button)
{
    if (!params.contains(QStringLiteral("button"))) {
        button = Qt::LeftButton;
        return true;
    }
    const QJsonValue value = params.value(QStringLiteral("button"));
    if (value.isDouble()) {
        button = Qt::MouseButton(int(value.toDouble()));
        return true;
    }
    if (!value.isString()) {
        return false;
    }
    const QString name = value.toString().trimmed().toLower();
    if (name == QStringLiteral("left")) {
        button = Qt::LeftButton;
    } else if (name == QStringLiteral("right")) {
        button = Qt::RightButton;
    } else if (name == QStringLiteral("middle")) {
        button = Qt::MiddleButton;
    } else {
        return false;
    }
    return true;
}

bool parseModifiers(const QJsonValue& value, Qt::KeyboardModifiers& mods)
{
    if (value.isUndefined() || value.isNull()) {
        mods = Qt::NoModifier;
        return true;
    }
    if (value.isDouble()) {
        mods = Qt::KeyboardModifiers(int(value.toDouble()));
        return true;
    }
    if (!value.isString()) {
        return false;
    }
    Qt::KeyboardModifiers parsed = Qt::NoModifier;
    const QString text = value.toString().trimmed();
    if (!text.isEmpty()) {
        const QStringList names = text.split(QLatin1Char('+'), Qt::SkipEmptyParts);
        for (const QString& raw : names) {
            const QString name = raw.trimmed().toLower();
            if (name == QStringLiteral("shift")) {
                parsed |= Qt::ShiftModifier;
            } else if (name == QStringLiteral("ctrl") || name == QStringLiteral("control")) {
                parsed |= Qt::ControlModifier;
            } else if (name == QStringLiteral("alt")) {
                parsed |= Qt::AltModifier;
            } else if (name == QStringLiteral("meta") || name == QStringLiteral("cmd")
                       || name == QStringLiteral("command")) {
                parsed |= Qt::MetaModifier;
            } else {
                return false;
            }
        }
    }
    mods = parsed;
    return true;
}

bool keyFromName(const QString& name, Qt::Key& key)
{
    if (name.isEmpty()) {
        return false;
    }
    if (name.size() == 1) {
        const QChar character = name.at(0).toUpper();
        if ((character >= QLatin1Char('A') && character <= QLatin1Char('Z'))
            || (character >= QLatin1Char('0') && character <= QLatin1Char('9'))) {
            key = Qt::Key(character.unicode());
            return true;
        }
        return false;
    }
    const QString lower = name.toLower();
    if (lower == QStringLiteral("enter") || lower == QStringLiteral("return")) {
        key = Qt::Key_Return;
    } else if (lower == QStringLiteral("escape") || lower == QStringLiteral("esc")) {
        key = Qt::Key_Escape;
    } else if (lower == QStringLiteral("tab")) {
        key = Qt::Key_Tab;
    } else if (lower == QStringLiteral("space")) {
        key = Qt::Key_Space;
    } else if (lower == QStringLiteral("delete") || lower == QStringLiteral("del")) {
        key = Qt::Key_Delete;
    } else if (lower == QStringLiteral("backspace")) {
        key = Qt::Key_Backspace;
    } else if (lower == QStringLiteral("home")) {
        key = Qt::Key_Home;
    } else if (lower == QStringLiteral("end")) {
        key = Qt::Key_End;
    } else if (lower == QStringLiteral("pageup")) {
        key = Qt::Key_PageUp;
    } else if (lower == QStringLiteral("pagedown")) {
        key = Qt::Key_PageDown;
    } else if (lower == QStringLiteral("up") || lower == QStringLiteral("arrowup")) {
        key = Qt::Key_Up;
    } else if (lower == QStringLiteral("down") || lower == QStringLiteral("arrowdown")) {
        key = Qt::Key_Down;
    } else if (lower == QStringLiteral("left") || lower == QStringLiteral("arrowleft")) {
        key = Qt::Key_Left;
    } else if (lower == QStringLiteral("right") || lower == QStringLiteral("arrowright")) {
        key = Qt::Key_Right;
    } else if (lower.startsWith(QLatin1Char('f'))) {
        bool ok = false;
        const int number = lower.mid(1).toInt(&ok);
        if (!ok || number < 1 || number > 35) {
            return false;
        }
        key = Qt::Key(int(Qt::Key_F1) + number - 1);
    } else {
        return false;
    }
    return true;
}

} // namespace

QJsonObject ControlServer::methodPointer(const QJsonObject& params)
{
    const QString op = params.value(QStringLiteral("op")).toString();
    const bool knownOp = op == QStringLiteral("click") || op == QStringLiteral("dblclick")
                         || op == QStringLiteral("move") || op == QStringLiteral("drag")
                         || op == QStringLiteral("scroll") || op == QStringLiteral("down")
                         || op == QStringLiteral("up");
    if (!knownOp) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown op: %1").arg(op));
    }
    const QString space = params.value(QStringLiteral("space")).toString();
    if (space != QStringLiteral("window") && space != QStringLiteral("image")) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("space must be window or image"));
    }
    double x = 0.0;
    double y = 0.0;
    if (!numberParam(params, QStringLiteral("x"), x)
        || !numberParam(params, QStringLiteral("y"), y)) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("x and y must be numeric"));
    }
    Qt::MouseButton button = Qt::LeftButton;
    if (!parseButton(params, button)) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("unknown button"));
    }
    Qt::KeyboardModifiers modifiers = Qt::NoModifier;
    if (!parseModifiers(params.value(QStringLiteral("modifiers")), modifiers)) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("unknown modifier"));
    }

    // A hostile `steps` cannot spin the GUI thread: the loop is bounded here.
    int steps = 1;
    double x2 = 0.0;
    double y2 = 0.0;
    if (op == QStringLiteral("drag")) {
        steps = 8;
        if (params.contains(QStringLiteral("steps"))
            && !intParam(params, QStringLiteral("steps"), steps)) {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("steps must be an integer"));
        }
        if (steps < 1 || steps > 256) {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("drag steps must be in 1..256"));
        }
        if (!numberParam(params, QStringLiteral("x2"), x2)
            || !numberParam(params, QStringLiteral("y2"), y2)) {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("drag requires numeric x2 and y2"));
        }
    } else if (op == QStringLiteral("scroll")) {
        steps = 1;
        if (params.contains(QStringLiteral("steps"))
            && !intParam(params, QStringLiteral("steps"), steps)) {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("steps must be an integer"));
        }
        if (steps < 1 || steps > 256) {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("scroll steps must be in 1..256"));
        }
    }

    struct Mapped {
        QPointF local;
        QPointF global;
    };
    QWidget* target = nullptr;
    std::function<Mapped(double, double)> mapPoint;
    if (space == QStringLiteral("image")) {
        ImageView* view = frame_->imageView();
        if (!view) {
            return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
        }
        target = view;
        mapPoint = [view](double ix, double iy) {
            const QPointF local = view->imageToWidget(QPointF(ix, iy));
            return Mapped{local, QPointF(view->mapToGlobal(local.toPoint()))};
        };
    } else {
        const QPoint framePoint(qRound(x), qRound(y));
        target = frame_->childAt(framePoint);
        if (!target) {
            target = frame_;
        }
        mapPoint = [this, target](double ix, double iy) {
            const QPoint framePoint(qRound(ix), qRound(iy));
            return Mapped{QPointF(target->mapFrom(frame_, framePoint)),
                          QPointF(frame_->mapToGlobal(framePoint))};
        };
    }

    const Qt::MouseButtons heldButton(button);
    auto sendMouse = [](QEvent::Type type, QWidget* widget, const QPointF& local,
                        const QPointF& global, Qt::MouseButton eventButton,
                        Qt::MouseButtons eventButtons, Qt::KeyboardModifiers mods) {
        QMouseEvent event(type, local, global, eventButton, eventButtons, mods);
        QApplication::sendEvent(widget, &event);
    };

    const Mapped first = mapPoint(x, y);
    if (op == QStringLiteral("click")) {
        sendMouse(QEvent::MouseButtonPress, target, first.local, first.global, button, heldButton,
                  modifiers);
        sendMouse(QEvent::MouseButtonRelease, target, first.local, first.global, button,
                  Qt::NoButton, modifiers);
    } else if (op == QStringLiteral("dblclick")) {
        sendMouse(QEvent::MouseButtonPress, target, first.local, first.global, button, heldButton,
                  modifiers);
        sendMouse(QEvent::MouseButtonRelease, target, first.local, first.global, button,
                  Qt::NoButton, modifiers);
        sendMouse(QEvent::MouseButtonDblClick, target, first.local, first.global, button,
                  heldButton, modifiers);
        sendMouse(QEvent::MouseButtonRelease, target, first.local, first.global, button,
                  Qt::NoButton, modifiers);
    } else if (op == QStringLiteral("down")) {
        heldButtons_ = heldButton;
        sendMouse(QEvent::MouseButtonPress, target, first.local, first.global, button, heldButton,
                  modifiers);
    } else if (op == QStringLiteral("up")) {
        heldButtons_ = Qt::NoButton;
        sendMouse(QEvent::MouseButtonRelease, target, first.local, first.global, button,
                  Qt::NoButton, modifiers);
    } else if (op == QStringLiteral("move")) {
        sendMouse(QEvent::MouseMove, target, first.local, first.global, Qt::NoButton,
                  heldButtons_, modifiers);
    } else if (op == QStringLiteral("drag")) {
        const Mapped last = mapPoint(x2, y2);
        sendMouse(QEvent::MouseButtonPress, target, first.local, first.global, button, heldButton,
                  modifiers);
        for (int i = 1; i <= steps; ++i) {
            const double t = double(i) / double(steps);
            const Mapped mid = mapPoint(x + (x2 - x) * t, y + (y2 - y) * t);
            sendMouse(QEvent::MouseMove, target, mid.local, mid.global, Qt::NoButton, heldButton,
                      modifiers);
        }
        sendMouse(QEvent::MouseButtonRelease, target, last.local, last.global, button,
                  Qt::NoButton, modifiers);
    } else { // scroll
        QWheelEvent event(first.local, first.global, QPoint(0, 0), QPoint(0, 120 * steps),
                          Qt::NoButton, modifiers, Qt::NoScrollPhase, false);
        QApplication::sendEvent(target, &event);
    }

    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    return ok(result);
}

QJsonObject ControlServer::methodKey(const QJsonObject& params)
{
    if (!params.value(QStringLiteral("sequence")).isString()) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("sequence must be a string"));
    }
    const QString sequence = params.value(QStringLiteral("sequence")).toString();
    const QStringList tokens = sequence.split(QLatin1Char('+'));
    if (tokens.isEmpty()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("sequence is empty"));
    }
    Qt::KeyboardModifiers modifiers = Qt::NoModifier;
    for (int i = 0; i + 1 < tokens.size(); ++i) {
        const QString name = tokens.at(i).trimmed().toLower();
        if (name == QStringLiteral("ctrl") || name == QStringLiteral("control")) {
            modifiers |= Qt::ControlModifier;
        } else if (name == QStringLiteral("shift")) {
            modifiers |= Qt::ShiftModifier;
        } else if (name == QStringLiteral("alt")) {
            modifiers |= Qt::AltModifier;
        } else if (name == QStringLiteral("meta") || name == QStringLiteral("cmd")
                   || name == QStringLiteral("command")) {
            modifiers |= Qt::MetaModifier;
        } else {
            return error(QStringLiteral("invalid_param"),
                         QStringLiteral("unknown modifier: %1").arg(name));
        }
    }
    Qt::Key key = Qt::Key_unknown;
    const QString keyName = tokens.last().trimmed();
    if (!keyFromName(keyName, key)) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown key: %1").arg(keyName));
    }

    QWidget* target = QApplication::focusWidget();
    if (!target) {
        target = frame_;
    }
    QKeyEvent press(QEvent::KeyPress, key, modifiers);
    QApplication::sendEvent(target, &press);
    QKeyEvent release(QEvent::KeyRelease, key, modifiers);
    QApplication::sendEvent(target, &release);

    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    return ok(result);
}

} // namespace pictura
