#pragma once

#include <QtCore/QFile>
#include <QtCore/QHash>
#include <QtCore/QSet>
#include <QtGui/QAction>
#include <QtTest/QtTest>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

#include <functional>

#include "adjustment_catalogue.h"
#include "commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object/command_catalogue.cxxqt.h"

namespace pictura::test {

inline void crMenuCatalogue(pictura::PicturaMainWindow& window)
{
    const ::rust::Vec<pictura::CommandSpecRow> rows = pictura::command_catalogue();
    const QList<pictura::CommandInfo> infos = window.registry()->describe();
    QHash<QString, pictura::CommandInfo> byId;
    for (const pictura::CommandInfo& info : infos) {
        byId.insert(info.id, info);
    }

    int catalogueCommands = 0;
    for (const pictura::CommandSpecRow& row : rows) {
        if (row.id.isEmpty()) {
            continue;
        }
        ++catalogueCommands;
        QVERIFY2(byId.contains(row.id), qPrintable(row.id));
        const pictura::CommandInfo info = byId.value(row.id);
        QCOMPARE(info.path, row.path);
        QCOMPARE(info.label, row.label);
    }
    QCOMPARE(infos.size(), catalogueCommands);

    QSet<QString> menuIds;
    std::function<void(QMenu*)> visit = [&](QMenu* menu) {
        for (QAction* action : menu->actions()) {
            if (QMenu* submenu = action->menu()) {
                visit(submenu);
            } else if (!action->data().toString().isEmpty()) {
                menuIds.insert(action->data().toString());
            }
        }
    };
    for (QAction* action : window.menuBar()->actions()) {
        if (QMenu* menu = action->menu()) {
            visit(menu);
        }
    }
    // The UI-size steps are shortcut-only: the frame keeps them on the window
    // and takes their rows out of the Window menu.
    const QSet<QString> shortcutOnly = {
        QString::fromLatin1(pictura::command_ids::WindowUiEnlarge),
        QString::fromLatin1(pictura::command_ids::WindowUiDecrease),
        QString::fromLatin1(pictura::command_ids::WindowUiReset)};
    for (const pictura::CommandSpecRow& row : rows) {
        if (!row.id.isEmpty() && !shortcutOnly.contains(row.id)) {
            QVERIFY2(menuIds.contains(row.id), qPrintable(row.id));
        }
    }
    QVERIFY(window.registry()->action(QString::fromLatin1(pictura::command_ids::FileNew))
            != nullptr);
}

inline void crIdentifiersStable(pictura::PicturaMainWindow& window)
{
    pictura::CommandRegistry* registry = window.registry();
    QAction* action = registry->action(QString::fromLatin1(pictura::command_ids::SelectAll));
    QVERIFY(action != nullptr);
    const QString originalText = action->text();
    const QList<QKeySequence> originalShortcuts =
        registry->shortcuts(QString::fromLatin1(pictura::command_ids::SelectAll));

    action->setText(QStringLiteral("Renamed Select All"));
    registry->setShortcuts(QString::fromLatin1(pictura::command_ids::SelectAll),
                           {QKeySequence(QStringLiteral("Ctrl+Alt+F12"))});
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectAll)));
    QVERIFY(window.activeView()->has_selection());

    registry->setShortcuts(QString::fromLatin1(pictura::command_ids::SelectAll),
                           originalShortcuts);
    action->setText(originalText);
    window.activeView()->deselect();
}

inline void crRustOnly()
{
    QVERIFY(QFINDTESTDATA("../../../../crates/pictura-app/cpp/command_tree.cpp").isEmpty());

    const QString commandsHeader =
        QFINDTESTDATA("../../../../crates/pictura-app/cpp/commands.h");
    QVERIFY(!commandsHeader.isEmpty());
    QFile header(commandsHeader);
    QVERIFY(header.open(QIODevice::ReadOnly));
    const QByteArray text = header.readAll();
    QVERIFY(!text.contains("namespace command_ids"));
    QVERIFY(text.contains("#include \"command_ids.h\""));
    QVERIFY(!QFINDTESTDATA("../../../../crates/pictura-app/cpp/command_ids.h").isEmpty());
}

inline void crListCommands(pictura::PicturaMainWindow& window)
{
    window.registry()->refresh();
    const QList<pictura::CommandInfo> infos = window.registry()->describe();
    QCOMPARE(infos.size(), 593);
    QHash<QString, pictura::CommandInfo> byId;
    for (const pictura::CommandInfo& info : infos) {
        byId.insert(info.id, info);
    }

    int index = 0;
    for (const pictura::CommandSpecRow& row : pictura::command_catalogue()) {
        if (row.id.isEmpty()) {
            continue;
        }
        QVERIFY(index < infos.size());
        const pictura::CommandInfo info = infos.at(index);
        ++index;
        QCOMPARE(info.id, row.id);
        QCOMPARE(info.path, row.path);
        QCOMPARE(info.label, row.label);
    }
    QCOMPARE(index, 593);
    QCOMPARE(index, infos.size());

    const pictura::CommandInfo fileNew =
        byId.value(QString::fromLatin1(pictura::command_ids::FileNew));
    QCOMPARE(fileNew.path, (QStringList{QStringLiteral("File"), QStringLiteral("New…")}));
    QCOMPARE(fileNew.label, QStringLiteral("New…"));
    QVERIFY(fileNew.implemented);
    QVERIFY(fileNew.enabled);

    const pictura::CommandInfo imageSize =
        byId.value(pictura::commandIdForPath(
            {QStringLiteral("Image"), QStringLiteral("Image Size…")}));
    QVERIFY(imageSize.implemented);
    QVERIFY(imageSize.enabled);

    const pictura::CommandInfo gaussian = byId.value(pictura::commandIdForPath(
        {QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Gaussian Blur")}));
    QVERIFY(gaussian.implemented);
    QVERIFY(gaussian.enabled);

    const pictura::CommandInfo extract = byId.value(pictura::commandIdForPath(
        {QStringLiteral("Filter"), QStringLiteral("Extract…")}));
    QVERIFY(!extract.implemented);

    const pictura::CommandInfo rgbMode =
        byId.value(QString::fromLatin1(pictura::command_ids::ImageModeRgb));
    QVERIFY(rgbMode.checked);
    const pictura::CommandInfo eightBits =
        byId.value(QString::fromLatin1(pictura::command_ids::ImageMode8Bits));
    QVERIFY(eightBits.checked);

    const pictura::CommandInfo hideLayers =
        byId.value(QString::fromLatin1(pictura::command_ids::LayerHideLayers));
    QCOMPARE(hideLayers.path,
             (QStringList{QStringLiteral("Layer"), QStringLiteral("Hide Layers")}));
}

// Image > Adjustments and the Image-root Auto commands are wired from the engine
// adjustment catalogue (#272): every catalogue command is implemented with a
// handler, dialog entries carry the ellipsis, and the menu keeps the catalogue's
// relative order.
inline void imageAdjustmentsFollowCatalogue(pictura::PicturaMainWindow& window)
{
    pictura::CommandRegistry* registry = window.registry();
    QVERIFY(registry != nullptr);
    registry->refresh();
    QMenu* image = nullptr;
    for (QAction* top : window.menuBar()->actions()) {
        if (top->text().remove(QLatin1Char('&')) == QStringLiteral("Image")) {
            image = top->menu();
        }
    }
    QVERIFY(image != nullptr);
    QMenu* adjustments = nullptr;
    for (QAction* action : image->actions()) {
        if (action->menu() && action->text().remove(QLatin1Char('&')) == QStringLiteral("Adjustments")) {
            adjustments = action->menu();
        }
    }
    QVERIFY(adjustments != nullptr);
    QStringList shown;
    for (QAction* action : adjustments->actions()) {
        if (!action->isSeparator()) {
            shown << action->text().remove(QStringLiteral("\u2026"));
        }
    }

    QSet<QString> implemented;
    for (const pictura::CommandInfo& info : registry->describe()) {
        if (info.implemented) {
            implemented.insert(info.id);
        }
    }
    int last = -1;
    int wired = 0;
    for (const pictura::AdjustmentKind& entry : pictura::adjustmentCatalogue()) {
        if (entry.image.isEmpty()) {
            continue;
        }
        const bool root = entry.image == QLatin1String("root");
        const QString id = root ? pictura::commandIdForPath({QStringLiteral("Image"), entry.title})
                                : pictura::commandIdForPath({QStringLiteral("Image"),
                                                             QStringLiteral("Adjustments"),
                                                             entry.title});
        QVERIFY2(implemented.contains(id), qPrintable(id));
        QAction* action = registry->action(id);
        QVERIFY2(action != nullptr, qPrintable(id));
        const bool dialog = entry.image == QLatin1String("dialog");
        QCOMPARE(action->text(), dialog ? entry.title + QStringLiteral("\u2026") : entry.title);
        ++wired;
        if (root) {
            continue;
        }
        const int at = shown.indexOf(entry.title);
        QVERIFY2(at >= 0, qPrintable(entry.title));
        if (dialog || entry.kind == QLatin1String("invert")) {
            QVERIFY2(at > last, qPrintable(entry.title + QStringLiteral(" keeps CS6 order")));
            last = at;
        }
    }
    QCOMPARE(wired, 22);
}

} // namespace pictura::test
