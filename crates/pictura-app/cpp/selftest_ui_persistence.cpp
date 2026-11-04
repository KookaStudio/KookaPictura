#include "selftest_ui_persistence.h"
#include "selftest_report.h"

#include "frame.h"
#include "options_bar.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "panels/panel_column.h"
#include "session.h"

#include "api/view_bridge.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonValue>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

namespace pictura {

namespace {

void pump(int count)
{
    for (int i = 0; i < count; ++i) {
        QCoreApplication::processEvents();
    }
}

QString suffixTextFor(PicturaMainWindow& frame, const QString& fieldName)
{
    auto* field = frame.findChild<NumericField*>(fieldName);
    if (!field) {
        return QString();
    }
    auto* suffix = field->findChild<QLabel*>(QStringLiteral("numericSuffix"));
    return suffix ? suffix->text() : QString();
}

} // namespace

int runUiPersistenceChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_column_railmode (324): a non-primary column toggled iconic keeps its
    // own mode across save/reload, and a v7 store without per-column rail modes
    // seeds every column from the legacy top-level `panelRailMode`.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        const bool madeLeft = frame.newColumnDropForTest(QStringLiteral("stylesPanel"),
                                                         QStringLiteral("left"));
        pump(4);
        pictura::PanelColumn* left = frame.columnForPanel(QStringLiteral("stylesPanel"));
        if (left) {
            left->setRailMode(true);
        }
        if (primary) {
            primary->setRailMode(false);
        }
        pump(4);
        frame.saveSession();
        const pictura::SessionState saved = pictura::loadSession();

        QString storedLeft;
        QString storedRight;
        for (const QJsonValue& value : saved.panelColumns) {
            const QJsonObject entry = value.toObject();
            if (entry.value(QStringLiteral("tools")).toBool()) {
                continue;
            }
            const QString side = entry.value(QStringLiteral("side")).toString();
            const QString mode = entry.value(QStringLiteral("railMode")).toString();
            if (side == QStringLiteral("left") && storedLeft.isEmpty()) {
                storedLeft = mode;
            } else if (side == QStringLiteral("right") && storedRight.isEmpty()) {
                storedRight = mode;
            }
        }
        const bool storedOk = saved.schemaVersion >= 9 && madeLeft
            && storedLeft == QStringLiteral("iconic")
            && storedRight == QStringLiteral("normal");

        frame.applyPanelSessionForTest(saved);
        pump(8);
        pictura::PanelColumn* restoredLeft = frame.columnForPanel(QStringLiteral("stylesPanel"));
        const bool appliedOk = frame.panelColumnCountForTest() == 3 && restoredLeft
            && restoredLeft->railMode() && !frame.panelColumn()->railMode();

        // Rewrite the two-column store as schema 7 (no per-column `railMode`) so
        // every column must seed its mode from the legacy top-level value.
        frame.saveSession();
        {
            QFile file(pictura::sessionFilePath());
            if (file.open(QIODevice::ReadOnly)) {
                QJsonObject obj = QJsonDocument::fromJson(file.readAll()).object();
                file.close();
                obj.insert(QStringLiteral("schemaVersion"), 7);
                obj.insert(QStringLiteral("panelRailMode"), QStringLiteral("iconic"));
                QJsonArray columns = obj.value(QStringLiteral("panelColumns")).toArray();
                for (int i = 0; i < columns.size(); ++i) {
                    QJsonObject entry = columns.at(i).toObject();
                    entry.remove(QStringLiteral("railMode"));
                    columns.replace(i, entry);
                }
                obj.insert(QStringLiteral("panelColumns"), columns);
                QFile out(pictura::sessionFilePath());
                if (out.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
                    out.write(QJsonDocument(obj).toJson(QJsonDocument::Compact));
                    out.close();
                }
            }
        }
        frame.applyPanelSessionForTest(pictura::loadSession());
        pump(8);
        int widgetColumnCount = 0;
        bool allIconic = true;
        for (pictura::PanelColumn* column : frame.panelColumns()) {
            if (!column || column->isToolsColumn()) {
                continue;
            }
            ++widgetColumnCount;
            if (!column->railMode()) {
                allIconic = false;
            }
        }
        allIconic = allIconic && widgetColumnCount == 2;
        ST_BEGIN("lpr_column_railmode");
        ST_PASS("lpr_column_railmode stored=%s/%s applied=%d seeded=%d columns=%d",
                qPrintable(storedLeft), qPrintable(storedRight), appliedOk ? 1 : 0,
                allIconic ? 1 : 0, frame.panelColumnCountForTest());
        if (!storedOk || !appliedOk || !allIconic) {
            return pictura::selfTest().fail(324, "per-column rail mode");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lpn_paint_percent (325): the paint options fields, and the Brush Preset
    // picker's Hardness, show a `%` suffix.
    {
        auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
        auto* hardnessBox = bar ? bar->brushPicker()->findChild<QSpinBox*>(
                                      QStringLiteral("brushPickerHardness"))
                                : nullptr;
        const QString hardness = hardnessBox ? hardnessBox->suffix() : QString();
        const QString opacity = suffixTextFor(frame, QStringLiteral("optionsBrushOpacity"));
        const QString flow = suffixTextFor(frame, QStringLiteral("optionsBrushFlow"));
        const bool percentOk = hardness == QStringLiteral("%")
            && opacity == QStringLiteral("%") && flow == QStringLiteral("%");
        ST_BEGIN("lpn_paint_percent");
        ST_PASS("lpn_paint_percent hardness=%s opacity=%s flow=%s", qPrintable(hardness),
                qPrintable(opacity), qPrintable(flow));
        if (!percentOk) {
            return pictura::selfTest().fail(325, "paint field percent suffix");
        }
    }

    // lpn_feather_no_popup (326): the Feather control is built without a slider
    // popup, so neither the arrow nor the popup widget exists.
    {
        auto* feather = frame.findChild<NumericField*>(QStringLiteral("optionsFeather"));
        auto* popup = feather
            ? feather->findChild<QWidget*>(QStringLiteral("numericPopup"))
            : nullptr;
        auto* arrow = feather
            ? feather->findChild<QToolButton*>(QStringLiteral("numericArrow"))
            : nullptr;
        const bool noPopup = feather && popup == nullptr && arrow == nullptr;
        ST_BEGIN("lpn_feather_no_popup");
        ST_PASS("lpn_feather_no_popup popup=%d arrow=%d", popup ? 1 : 0, arrow ? 1 : 0);
        if (!noPopup) {
            return pictura::selfTest().fail(326, "feather slider popup");
        }
    }

    // lpn_px_tight (327): a `px` suffix sits immediately after the value with no
    // space, both on the configured Feather field and on a fresh control.
    {
        const QString featherSuffix = suffixTextFor(frame, QStringLiteral("optionsFeather"));
        NumericFieldConfig config;
        config.minimum = 0.0;
        config.maximum = 100.0;
        config.step = 1.0;
        config.decimals = 0;
        config.suffix = QStringLiteral("px");
        config.namePrefix = QStringLiteral("pxTight");
        NumericField field(QStringLiteral("W"), config);
        field.setValue(12);
        auto* edit = field.findChild<QLineEdit*>(QStringLiteral("pxTightEdit"));
        auto* suffix = field.findChild<QLabel*>(QStringLiteral("pxTightSuffix"));
        const bool tightOk = featherSuffix == QStringLiteral("px") && edit && suffix
            && suffix->text() == QStringLiteral("px")
            && edit->text() + suffix->text() == QStringLiteral("12px");
        ST_BEGIN("lpn_px_tight");
        ST_PASS("lpn_px_tight feather=%s value=%s combined=%s", qPrintable(featherSuffix),
                edit ? qPrintable(edit->text()) : "-",
                (edit && suffix) ? qPrintable(edit->text() + suffix->text()) : "-");
        if (!tightOk) {
            return pictura::selfTest().fail(327, "px suffix spacing");
        }
    }

    return 0;
}

} // namespace pictura
