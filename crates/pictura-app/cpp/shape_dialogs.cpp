// The shape tools' dialogs: the live-shape conversion prompt and the Create
// dialogs a click (rather than a drag) opens. Field names and layout follow
// Photoshop CC's dialogs, which is where both first appear; CS6 had neither
// live shapes nor per-corner radii.

#include "shape_dialogs.h"

#include "preferences.h"
#include "tools.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

bool confirmLiveShapeToPath(QWidget* parent, bool* asked)
{
    const auto prefs = loadPreferences();
    if (asked) {
        *asked = prefs.confirmLiveShapeToPath;
    }
    if (!prefs.confirmLiveShapeToPath) {
        return true;
    }
    QMessageBox box(QMessageBox::Information, QStringLiteral("Kooka Pictura"),
                    QObject::tr("This operation will turn a live shape into a regular "
                                "path. Continue?"),
                    QMessageBox::Yes | QMessageBox::No, parent);
    box.setObjectName(QStringLiteral("liveShapeToPathPrompt"));
    box.setDefaultButton(QMessageBox::Yes);
    auto* dontShow = new QCheckBox(QObject::tr("Don't show again"), &box);
    dontShow->setObjectName(QStringLiteral("liveShapeDontShowAgain"));
    box.setCheckBox(dontShow);
    const bool yes = box.exec() == QMessageBox::Yes;
    if (yes && dontShow->isChecked()) {
        const auto before = loadPreferences();
        auto after = before;
        after.confirmLiveShapeToPath = false;
        savePreferences(before, after);
    }
    return yes;
}

namespace {

QDoubleSpinBox* pixelField(const QString& name, double value, double minimum, QWidget* parent)
{
    auto* field = new QDoubleSpinBox(parent);
    field->setObjectName(name);
    field->setDecimals(2);
    field->setRange(minimum, 300000.0);
    field->setSuffix(QStringLiteral(" px"));
    field->setValue(value);
    return field;
}

QString title(ToolId id)
{
    switch (id) {
    case ToolId::RoundedRectangle:
        return QObject::tr("Create Rounded Rectangle");
    case ToolId::Ellipse:
        return QObject::tr("Create Ellipse");
    case ToolId::Polygon:
        return QObject::tr("Create Polygon");
    case ToolId::CustomShape:
        return QObject::tr("Create Custom Shape");
    default:
        return QObject::tr("Create Rectangle");
    }
}

} // namespace

bool execCreateShapeDialog(ToolId id, CreateShapeValues& values, QWidget* parent)
{
    QDialog dialog(parent);
    dialog.setObjectName(QStringLiteral("createShapeDialog"));
    dialog.setWindowTitle(title(id));
    auto* layout = new QVBoxLayout(&dialog);

    auto* size = new QGridLayout;
    auto* width = pixelField(QStringLiteral("createShapeWidth"), values.width, 0.01, &dialog);
    auto* height = pixelField(QStringLiteral("createShapeHeight"), values.height, 0.01, &dialog);
    size->addWidget(new QLabel(QObject::tr("Width:"), &dialog), 0, 0);
    size->addWidget(width, 0, 1);
    size->addWidget(new QLabel(QObject::tr("Height:"), &dialog), 0, 2);
    size->addWidget(height, 0, 3);
    layout->addLayout(size);

    // The radii sit where their corners are; the glyph beside each names it.
    std::array<QDoubleSpinBox*, 4> radii{};
    if (id == ToolId::RoundedRectangle) {
        layout->addWidget(new QLabel(QObject::tr("Radii:"), &dialog));
        auto* grid = new QGridLayout;
        const std::array<const char*, 4> names{"createShapeRadiusTopLeft",
                                               "createShapeRadiusTopRight",
                                               "createShapeRadiusBottomRight",
                                               "createShapeRadiusBottomLeft"};
        const std::array<QString, 4> glyphs{QStringLiteral("╭"), QStringLiteral("╮"),
                                            QStringLiteral("╯"), QStringLiteral("╰")};
        // Grid cells (row, field column, glyph column) for TL, TR, BR, BL.
        const std::array<std::array<int, 3>, 4> cells{{{0, 1, 0}, {0, 2, 3}, {1, 2, 3}, {1, 1, 0}}};
        for (int i = 0; i < 4; ++i) {
            radii[i] = pixelField(QString::fromLatin1(names[i]), values.radii[i], 0.0, &dialog);
            grid->addWidget(new QLabel(glyphs[i], &dialog), cells[i][0], cells[i][2]);
            grid->addWidget(radii[i], cells[i][0], cells[i][1]);
        }
        layout->addLayout(grid);
    }

    QSpinBox* sides = nullptr;
    QCheckBox* smoothCorners = nullptr;
    QCheckBox* star = nullptr;
    QDoubleSpinBox* indent = nullptr;
    QCheckBox* smoothIndents = nullptr;
    if (id == ToolId::Polygon) {
        auto* row = new QGridLayout;
        sides = new QSpinBox(&dialog);
        sides->setObjectName(QStringLiteral("createShapeSides"));
        sides->setRange(3, 100);
        sides->setValue(values.sides);
        row->addWidget(new QLabel(QObject::tr("Number of Sides:"), &dialog), 0, 0);
        row->addWidget(sides, 0, 1);
        layout->addLayout(row);
        smoothCorners = new QCheckBox(QObject::tr("Smooth Corners"), &dialog);
        smoothCorners->setObjectName(QStringLiteral("createShapeSmoothCorners"));
        smoothCorners->setChecked(values.smoothCorners);
        star = new QCheckBox(QObject::tr("Star"), &dialog);
        star->setObjectName(QStringLiteral("createShapeStar"));
        star->setChecked(values.star);
        indent = new QDoubleSpinBox(&dialog);
        indent->setObjectName(QStringLiteral("createShapeIndent"));
        indent->setRange(1.0, 99.0);
        indent->setDecimals(0);
        indent->setSuffix(QStringLiteral("%"));
        indent->setValue(values.indent);
        smoothIndents = new QCheckBox(QObject::tr("Smooth Indents"), &dialog);
        smoothIndents->setObjectName(QStringLiteral("createShapeSmoothIndents"));
        smoothIndents->setChecked(values.smoothIndents);
        auto* indentLabel = new QLabel(QObject::tr("Indent Sides By:"), &dialog);
        auto* indentRow = new QGridLayout;
        indentRow->addWidget(indentLabel, 0, 0);
        indentRow->addWidget(indent, 0, 1);
        layout->addWidget(smoothCorners);
        layout->addWidget(star);
        layout->addLayout(indentRow);
        layout->addWidget(smoothIndents);
        // Indent and Smooth Indents only mean something for a star.
        const auto syncStar = [=](bool on) {
            indentLabel->setEnabled(on);
            indent->setEnabled(on);
            smoothIndents->setEnabled(on);
        };
        syncStar(values.star);
        QObject::connect(star, &QCheckBox::toggled, &dialog, syncStar);
    }

    auto* fromCenter = new QCheckBox(QObject::tr("From Center"), &dialog);
    fromCenter->setObjectName(QStringLiteral("createShapeFromCenter"));
    fromCenter->setChecked(values.fromCenter);
    layout->addWidget(fromCenter);

    auto* buttons =
        new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);
    width->setFocus();
    width->selectAll();

    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    values.width = width->value();
    values.height = height->value();
    values.fromCenter = fromCenter->isChecked();
    for (int i = 0; i < 4; ++i) {
        if (radii[i]) {
            values.radii[i] = radii[i]->value();
        }
    }
    if (sides) {
        values.sides = sides->value();
        values.smoothCorners = smoothCorners->isChecked();
        values.star = star->isChecked();
        values.indent = indent->value();
        values.smoothIndents = smoothIndents->isChecked();
    }
    return true;
}

} // namespace pictura
