#include "menu_registrar.h"

#include "api/engine_bridge.h"
#include "api/paint_bridge.h"
#include "commands.h"
#include "document_registry.h"
#include "tools.h"

#include <QtCore/QCryptographicHash>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLocale>
#include <QtCore/QMap>
#include <QtCore/QSaveFile>
#include <QtCore/QStandardPaths>
#include <QtCore/QVariant>
#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>

#include <cmath>
#include <limits>

namespace pictura {
namespace {

struct ScaleValue {
    int pixels;
    double logical;
    QString units;

    bool operator==(const ScaleValue& other) const
    {
        return pixels == other.pixels && logical == other.logical && units == other.units;
    }
};

QString numericText(double value)
{
    return QString::number(value, 'g', std::numeric_limits<double>::max_digits10);
}

ScaleValue activeCustomScale(const PictureView& view)
{
    const auto values = measurement_scale_values(view);
    if (values.size() == 2) {
        return {int(values[0]), values[1], measurement_scale_units(view)};
    }
    return {0, 1.0, QStringLiteral("px")};
}

QDir presetDirectory()
{
    return QDir(QDir(QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation))
                    .filePath(QStringLiteral("kooka-pictura/Presets/Measurement Scales")));
}

QString presetFilename(const QString& name)
{
    const auto hash = QCryptographicHash::hash(name.toUtf8(), QCryptographicHash::Sha256).toHex();
    return QString::fromLatin1(hash) + QStringLiteral(".json");
}

QMap<QString, ScaleValue> presets()
{
    QMap<QString, ScaleValue> result;
    const auto dir = presetDirectory();
    for (const auto& filename : dir.entryList({QStringLiteral("*.json")}, QDir::Files)) {
        QFile file(dir.filePath(filename));
        if (!file.open(QIODevice::ReadOnly)) {
            continue;
        }
        const auto data = QJsonDocument::fromJson(file.readAll()).object();
        const auto name = data.value(QStringLiteral("name")).toString();
        const ScaleValue value{data.value(QStringLiteral("pixels")).toInt(),
                               data.value(QStringLiteral("logical")).toDouble(),
                               data.value(QStringLiteral("units")).toString()};
        if (!name.trimmed().isEmpty() && presetFilename(name) == filename
            && measurement_scale_valid(value.pixels, value.logical, value.units)) {
            result.insert(name, value);
        }
    }
    return result;
}

bool savePreset(const QString& name, const ScaleValue& scale)
{
    const auto dir = presetDirectory();
    if (!QDir().mkpath(dir.absolutePath())) {
        return false;
    }
    const QJsonObject object{{QStringLiteral("name"), name},
                             {QStringLiteral("pixels"), scale.pixels},
                             {QStringLiteral("logical"), scale.logical},
                             {QStringLiteral("units"), scale.units}};
    const auto bytes = QJsonDocument(object).toJson(QJsonDocument::Compact);
    QSaveFile file(dir.filePath(presetFilename(name)));
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size() && file.commit();
}

} // namespace

void MenuRegistrar::wireMeasurementScale()
{
    if (!edition_is_extended()) {
        return;
    }
    commands_->setHandler(command_ids::ImageAnalysisSetMeasurementScaleDefault, [this]() {
        if (auto* view = activeView()) {
            reset_measurement_scale(*view);
        }
        refreshMeasurementScaleChecks();
    });
    commands_->setHandler(command_ids::ImageAnalysisSetMeasurementScaleCustom,
                          [this]() { openMeasurementScaleDialog(); });
    for (const auto* id : {command_ids::ImageAnalysisSetMeasurementScaleDefault,
                           command_ids::ImageAnalysisSetMeasurementScaleCustom}) {
        commands_->setEnabledProvider(id, [this]() {
            auto* view = activeView();
            return view && view->has_document();
        });
    }
}

void MenuRegistrar::buildMeasurementScaleMenu()
{
    auto* defaultAction = commands_->action(command_ids::ImageAnalysisSetMeasurementScaleDefault);
    auto* customAction = commands_->action(command_ids::ImageAnalysisSetMeasurementScaleCustom);
    auto* menu = defaultAction ? qobject_cast<QMenu*>(defaultAction->parent()) : nullptr;
    if (!menu || !customAction) {
        return;
    }
    measurementScaleMenu_ = menu;
    menu->setObjectName(QStringLiteral("measurementScaleMenu"));
    connect(menu, &QMenu::aboutToShow, this, &MenuRegistrar::reloadMeasurementScalePresets);
    reloadMeasurementScalePresets();
}

void MenuRegistrar::reloadMeasurementScalePresets()
{
    auto* menu = measurementScaleMenu_.data();
    if (!menu) {
        return;
    }
    for (auto* action : menu->actions()) {
        if (action->property("measurementScalePreset").isValid()) {
            delete action;
        }
    }
    const auto saved = presets();
    for (auto it = saved.cbegin(); it != saved.cend(); ++it) {
        const auto value = it.value();
        QString label = it.key();
        label.replace(QLatin1Char('&'), QStringLiteral("&&"));
        auto* action = menu->addAction(label);
        action->setProperty("measurementScalePreset",
                            QVariantList{value.pixels, value.logical, value.units});
        action->setCheckable(true);
        connect(action, &QAction::triggered, this, [this, value]() {
            if (auto* target = activeView()) {
                apply_measurement_scale(*target, value.pixels, value.logical, value.units);
            }
            refreshMeasurementScaleChecks();
        });
    }
    refreshMeasurementScaleChecks();
}

void MenuRegistrar::refreshMeasurementScaleChecks()
{
    if (!measurementScaleMenu_) {
        return;
    }
    auto* view = activeView();
    const bool hasDocument = view && view->has_document();
    const auto active = hasDocument ? activeCustomScale(*view) : ScaleValue{};
    auto* defaultAction = commands_->action(command_ids::ImageAnalysisSetMeasurementScaleDefault);
    auto* customAction = commands_->action(command_ids::ImageAnalysisSetMeasurementScaleCustom);
    bool matched = false;
    for (auto* action : measurementScaleMenu_->actions()) {
        const auto values = action->property("measurementScalePreset").toList();
        if (values.size() != 3) {
            continue;
        }
        const ScaleValue preset{values[0].toInt(), values[1].toDouble(), values[2].toString()};
        const bool checked = !matched && preset == active;
        action->setEnabled(customAction->isEnabled());
        action->setChecked(checked);
        matched |= checked;
    }
    defaultAction->setChecked(hasDocument && active.pixels == 0);
    customAction->setChecked(active.pixels > 0 && !matched);
}

void MenuRegistrar::openMeasurementScaleDialog()
{
    auto* view = activeView();
    if (!view || !view->has_document() || !tools_ || !documents_) {
        return;
    }
    if (measurementScaleDialog_) {
        measurementScaleDialog_->raise();
        measurementScaleDialog_->activateWindow();
        return;
    }

    auto* dialog = new QDialog(dialogParent());
    measurementScaleDialog_ = dialog;
    dialog->setObjectName(QStringLiteral("measurementScaleDialog"));
    dialog->setWindowTitle(tr("Measurement Scale"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    const QPointer<PictureView> target(view);
    auto* layout = new QFormLayout(dialog);

    auto* pixels = new QLineEdit(dialog);
    auto* logical = new QLineEdit(dialog);
    auto* units = new QLineEdit(dialog);
    pixels->setObjectName(QStringLiteral("measurementPixelLength"));
    logical->setObjectName(QStringLiteral("measurementLogicalLength"));
    units->setObjectName(QStringLiteral("measurementLogicalUnits"));
    layout->addRow(tr("Pixel Length"), pixels);
    layout->addRow(tr("Logical Length"), logical);
    layout->addRow(tr("Logical Units"), units);

    auto* names = new QComboBox(dialog);
    names->setObjectName(QStringLiteral("measurementScalePresets"));
    names->setEditable(true);
    names->setInsertPolicy(QComboBox::NoInsert);
    layout->addRow(tr("Preset"), names);
    auto* save = new QPushButton(tr("Save Preset"), dialog);
    auto* erase = new QPushButton(tr("Delete Preset"), dialog);
    save->setObjectName(QStringLiteral("measurementScaleSavePreset"));
    erase->setObjectName(QStringLiteral("measurementScaleDeletePreset"));
    save->setAutoDefault(false);
    erase->setAutoDefault(false);
    layout->addRow(save, erase);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, dialog);
    auto* apply = buttons->button(QDialogButtonBox::Ok);
    apply->setObjectName(QStringLiteral("measurementScaleApply"));
    layout->addRow(buttons);

    const auto fill = [pixels, logical, units](const ScaleValue& value) {
        pixels->setText(QString::number(value.pixels == 0 ? 1 : value.pixels));
        logical->setText(numericText(value.logical));
        units->setText(value.units);
    };
    const auto draft = [pixels, logical, units]() {
        return ScaleValue{pixels->text().toInt(), QLocale::c().toDouble(logical->text()),
                          units->text()};
    };
    const auto reloadNames = [names](const QString& current) {
        names->clear();
        names->addItems(presets().keys());
        names->setEditText(current);
    };
    const auto validate = [draft, names, apply, save, erase]() {
        const auto value = draft();
        const bool valid = measurement_scale_valid(value.pixels, value.logical, value.units);
        const auto name = names->currentText().trimmed();
        apply->setEnabled(valid);
        save->setEnabled(valid && !name.isEmpty());
        erase->setEnabled(!name.isEmpty() && names->findText(name, Qt::MatchExactly) >= 0);
    };
    for (auto* input : {pixels, logical, units}) {
        connect(input, &QLineEdit::textChanged, dialog, validate);
    }
    connect(names, &QComboBox::editTextChanged, dialog, validate);
    connect(names, &QComboBox::textActivated, dialog, [fill](const QString& name) {
        const auto saved = presets();
        const auto it = saved.constFind(name);
        if (it != saved.constEnd()) {
            fill(it.value());
        }
    });
    connect(save, &QPushButton::clicked, dialog, [this, draft, names, reloadNames, validate]() {
        const auto name = names->currentText().trimmed();
        const auto value = draft();
        if (!name.isEmpty() && measurement_scale_valid(value.pixels, value.logical, value.units)
            && savePreset(name, value)) {
            reloadNames(name);
            validate();
            reloadMeasurementScalePresets();
        } else {
            showStatus(tr("Could not save the measurement scale preset."), 5000);
        }
    });
    connect(erase, &QPushButton::clicked, dialog, [this, names, reloadNames, validate]() {
        const auto name = names->currentText().trimmed();
        if (QFile::remove(presetDirectory().filePath(presetFilename(name)))) {
            reloadNames(QString());
            validate();
            reloadMeasurementScalePresets();
        } else {
            showStatus(tr("Could not delete the measurement scale preset."), 5000);
        }
    });
    connect(buttons, &QDialogButtonBox::rejected, dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, dialog, [this, dialog, target, draft]() {
        const auto value = draft();
        if (target && target == activeView()
            && measurement_scale_valid(value.pixels, value.logical, value.units)) {
            if (activeCustomScale(*target) == value
                || apply_measurement_scale(*target, value.pixels, value.logical, value.units)) {
                dialog->accept();
            }
        }
    });
    connect(documents_, &DocumentRegistry::activeDocumentChanged, dialog, [this, dialog, target]() {
        if (!target || target != activeView()) {
            dialog->reject();
        }
    });
    connect(view, &QObject::destroyed, dialog, &QDialog::reject);
    connect(dialog, &QDialog::finished, this, [this, dialog]() {
        dialog->setObjectName(QString());
        measurementScaleDialog_.clear();
    });
    connect(tools_, &ToolController::rulerChanged, dialog, [this, target, pixels]() {
        if (!target || target != activeView()) {
            return;
        }
        const auto values = ruler_measurement(*target);
        if (values.size() == 6 && values[5] > 0) {
            const double length = std::round(values[5]);
            if (length <= std::numeric_limits<int>::max()) {
                pixels->setText(QString::number(length < 1 ? 1 : int(length)));
            }
        }
    });

    fill(activeCustomScale(*view));
    reloadNames(QString());
    validate();
    tools_->setActiveTool(ToolId::Ruler);
    dialog->show();
}

} // namespace pictura
