#include "preferences_dialog.h"

#include "canvas_ruler.h"
#include "panels/numeric_field.h"
#include "session.h"
#include "theme.h"

#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

const QString PreferencesDialog::kGeneral = QStringLiteral("General");
const QString PreferencesDialog::kInterface = QStringLiteral("Interface");
const QString PreferencesDialog::kFileHandling = QStringLiteral("File Handling");
const QString PreferencesDialog::kPerformance = QStringLiteral("Performance");
const QString PreferencesDialog::kUnits = QStringLiteral("Units & Rulers");
const QString PreferencesDialog::kGuides = QStringLiteral("Guides, Grid, & Slices");

namespace {

// The CS6 pane list; the real pages are the ones present in `realPages_`.
const char* const kPaneNames[] = {
    "General",
    "Interface",
    "File Handling",
    "Performance",
    "Cursors",
    "Transparency & Gamut",
    "Units & Rulers",
    "Guides, Grid, & Slices",
    "Plug-ins",
    "Type",
    "3D",
};

struct NamedColor {
    const char* name;
    QColor color;
};

// CS6's guide colour presets. Cyan and Magenta were sampled from a CS6
// Preferences screenshot (#294) and Light Blue from its slice swatch.
// ponytail: the other presets are approximated in the same 74/255 family.
const NamedColor kGuideColors[] = {
    {"Light Blue", QColor(74, 113, 255)}, {"Light Red", QColor(255, 74, 74)},
    {"Green", QColor(74, 255, 74)},       {"Medium Blue", QColor(74, 74, 255)},
    {"Yellow", QColor(255, 255, 74)},     {"Magenta", QColor(255, 74, 255)},
    {"Cyan", QColor(74, 255, 255)},       {"Light Gray", QColor(192, 192, 192)},
    {"Black", QColor(0, 0, 0)},
};

QIcon swatchIcon(const QColor& color)
{
    QPixmap pixmap(12, 12);
    pixmap.fill(color);
    return QIcon(pixmap);
}

QComboBox* colorCombo(const QString& objectName, QWidget* parent)
{
    auto* combo = new QComboBox(parent);
    combo->setObjectName(objectName);
    for (const NamedColor& preset : kGuideColors) {
        combo->addItem(swatchIcon(preset.color), QString::fromUtf8(preset.name), preset.color);
    }
    combo->addItem(QObject::tr("Custom…"));
    return combo;
}

QLabel* swatch(const QColor& color, QWidget* parent)
{
    auto* label = new QLabel(parent);
    label->setFixedSize(40, 40);
    label->setAutoFillBackground(true);
    label->setStyleSheet(QStringLiteral("background: %1; border: 1px solid #808080;")
                             .arg(color.name()));
    return label;
}

} // namespace

PreferencesDialog::PreferencesDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("preferencesDialog"));
    setWindowTitle(tr("Preferences"));
    setWindowModality(Qt::NonModal);

    auto* layout = new QHBoxLayout(this);
    pageList_ = new QListWidget(this);
    pageList_->setObjectName(QStringLiteral("preferencesPages"));
    pageList_->setFixedWidth(160);
    layout->addWidget(pageList_);

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("preferencesStack"));
    layout->addWidget(stack_, 1);

    realPages_ = {kGeneral, kInterface, kFileHandling, kPerformance, kUnits, kGuides};

    // General: the app's real theme brightness is the only General setting.
    auto* generalPage = new QWidget(this);
    generalPage->setObjectName(QStringLiteral("preferencesGeneralPage"));
    auto* generalForm = new QFormLayout(generalPage);
    NumericFieldConfig brightnessConfig;
    brightnessConfig.minimum = 0;
    brightnessConfig.maximum = Theme::kLevelCount - 1;
    brightnessConfig.decimals = 0;
    brightnessConfig.objectName = QStringLiteral("preferencesBrightness");
    brightness_ = new NumericField(QString(), brightnessConfig, generalPage);
    brightness_->setValue(Theme::kDefaultLevel);
    generalForm->addRow(tr("Brightness"), brightness_);
    stack_->addWidget(generalPage);

    auto* interfacePage = new QWidget(this);
    interfacePage->setObjectName(QStringLiteral("preferencesInterfacePage"));
    auto* interfaceLayout = new QVBoxLayout(interfacePage);
    makeCheckbox(QStringLiteral("useShiftKeyForToolSwitch"),
                 tr("Use Shift Key For Tool Switch"), interfacePage);
    makeCheckbox(QStringLiteral("autoCollapseIconic"), tr("Auto-Collapse Iconic Panels"),
                 interfacePage);
    makeCheckbox(QStringLiteral("autoShowHidden"), tr("Auto-Show Hidden Panels"),
                 interfacePage);
    interfaceLayout->addStretch(1);
    stack_->addWidget(interfacePage);

    // File Handling: CS6's crash-recovery autosave, a checkbox and its interval
    // on one row.
    auto* fileHandlingPage = new QWidget(this);
    fileHandlingPage->setObjectName(QStringLiteral("preferencesFileHandlingPage"));
    auto* fileHandlingLayout = new QVBoxLayout(fileHandlingPage);
    auto* autoSaveRow = new QWidget(fileHandlingPage);
    auto* autoSaveLayout = new QHBoxLayout(autoSaveRow);
    autoSaveLayout->setContentsMargins(0, 0, 0, 0);
    makeCheckbox(QStringLiteral("autoSaveRecovery"),
                 tr("Automatically Save Recovery Information Every:"), autoSaveRow);
    autoSaveMinutes_ = new QComboBox(autoSaveRow);
    autoSaveMinutes_->setObjectName(QStringLiteral("preferencesAutoSaveMinutes"));
    for (int minutes : kAutoSaveMinuteChoices) {
        autoSaveMinutes_->addItem(minutes == 60 ? tr("1 Hour") : tr("%1 Minutes").arg(minutes),
                                  minutes);
    }
    autoSaveLayout->addWidget(autoSaveMinutes_);
    autoSaveLayout->addStretch(1);
    fileHandlingLayout->addWidget(autoSaveRow);
    fileHandlingLayout->addStretch(1);
    stack_->addWidget(fileHandlingPage);
    setAutoSaveRecovery(true, 10);
    connect(autoSaveMinutes_, &QComboBox::currentIndexChanged, this, [this] {
        emit autoSaveRecoveryChanged(checkbox(QStringLiteral("autoSaveRecovery"))->isChecked(),
                                     autoSaveMinutes_->currentData().toInt());
    });

    auto* performancePage = new QWidget(this);
    performancePage->setObjectName(QStringLiteral("preferencesPerformancePage"));
    auto* performanceLayout = new QVBoxLayout(performancePage);
    makeCheckbox(QStringLiteral("useGpuCompute"), tr("Use GPU Compute"), performancePage);
    performanceLayout->addStretch(1);
    stack_->addWidget(performancePage);
    stack_->addWidget(buildUnitsPage());
    stack_->addWidget(buildGuidesPage());

    // List rows carry their pane name in Qt::UserRole: the CS6 pane order does
    // not match the stack's real-page order, so the row maps by name.
    for (const char* name : kPaneNames) {
        const QString pageName = QString::fromUtf8(name);
        auto* item = new QListWidgetItem(pageName, pageList_);
        item->setData(Qt::UserRole, pageName);
        if (!realPages_.contains(pageName)) {
            item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
        }
    }
    connect(pageList_, &QListWidget::currentRowChanged, this, [this](int row) {
        const QListWidgetItem* item = pageList_->item(row);
        if (!item) {
            return;
        }
        const int index = realPages_.indexOf(item->data(Qt::UserRole).toString());
        if (index >= 0) {
            stack_->setCurrentIndex(index);
        }
    });
    connect(brightness_, &NumericField::valueChanged, this,
            [this](double level) { emit brightnessLevelChanged(qRound(level)); });
    pageList_->setCurrentRow(0);
}

QCheckBox* PreferencesDialog::makeCheckbox(const QString& key, const QString& label,
                                           QWidget* page)
{
    auto* box = new QCheckBox(label, page);
    box->setObjectName(QStringLiteral("preferences_") + key);
    page->layout()->addWidget(box);
    checkboxes_.insert(key, box);
    connect(box, &QCheckBox::toggled, this, [this, key](bool on) {
        if (key == QStringLiteral("useShiftKeyForToolSwitch")) {
            emit useShiftKeyForToolSwitchChanged(on);
        } else if (key == QStringLiteral("autoCollapseIconic")) {
            emit autoCollapseIconicChanged(on);
        } else if (key == QStringLiteral("autoShowHidden")) {
            emit autoShowHiddenChanged(on);
        } else if (key == QStringLiteral("useGpuCompute")) {
            emit gpuComputeChanged(on);
        } else if (key == QStringLiteral("autoSaveRecovery")) {
            autoSaveMinutes_->setEnabled(on);
            emit autoSaveRecoveryChanged(on, autoSaveMinutes_->currentData().toInt());
        }
    });
    return box;
}

QCheckBox* PreferencesDialog::checkbox(const QString& key) const
{
    return checkboxes_.value(key, nullptr);
}

void PreferencesDialog::setShiftKeyForToolSwitch(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useShiftKeyForToolSwitch"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setAutoCollapseIconic(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("autoCollapseIconic"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setAutoShowHidden(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("autoShowHidden"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setBrightnessLevel(int level)
{
    brightness_->setValue(level);
}

void PreferencesDialog::setGpuCompute(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useGpuCompute"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setGpuComputeEnabled(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useGpuCompute"))) {
        box->setEnabled(on);
    }
}

void PreferencesDialog::setAutoSaveRecovery(bool on, int minutes)
{
    QCheckBox* box = checkbox(QStringLiteral("autoSaveRecovery"));
    const QSignalBlocker blockBox(box);
    const QSignalBlocker blockMinutes(autoSaveMinutes_);
    box->setChecked(on);
    autoSaveMinutes_->setEnabled(on);
    const int index = autoSaveMinutes_->findData(minutes);
    autoSaveMinutes_->setCurrentIndex(index >= 0 ? index : autoSaveMinutes_->findData(10));
}

void PreferencesDialog::showPage(const QString& page)
{
    const int index = realPages_.indexOf(page);
    if (index < 0) {
        return;
    }
    for (int row = 0; row < pageList_->count(); ++row) {
        if (pageList_->item(row)->data(Qt::UserRole).toString() == page) {
            pageList_->setCurrentRow(row);
            break;
        }
    }
    stack_->setCurrentIndex(index);
}

void PreferencesDialog::openOn(const QString& page)
{
    showPage(page);
    show();
    raise();
    activateWindow();
}

QStringList PreferencesDialog::pagesForTest() const
{
    return realPages_;
}

QString PreferencesDialog::currentPageForTest() const
{
    return realPages_.value(stack_->currentIndex());
}

bool PreferencesDialog::setCheckboxForTest(const QString& key, bool on)
{
    QCheckBox* box = checkbox(key);
    if (!box) {
        return false;
    }
    box->setChecked(on);
    return true;
}

bool PreferencesDialog::checkboxForTest(const QString& key) const
{
    QCheckBox* box = checkbox(key);
    return box && box->isChecked();
}

// Units & Rulers: the ruler unit and Point/Pica Size are live; Type, Column
// Size, and New Document Preset Resolutions show CS6's defaults, disabled
// until something reads them.
QWidget* PreferencesDialog::buildUnitsPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("preferencesUnitsPage"));
    auto* layout = new QGridLayout(page);

    auto* units = new QGroupBox(tr("Units"), page);
    auto* unitsForm = new QFormLayout(units);
    rulerUnit_ = new QComboBox(units);
    rulerUnit_->setObjectName(QStringLiteral("preferencesRulerUnits"));
    for (int i = 0; i < kRulerUnitCount; ++i) {
        rulerUnit_->addItem(rulerUnitName(RulerUnit(i)));
    }
    rulerUnit_->setCurrentIndex(int(RulerUnit::Inches));
    auto* typeUnit = new QComboBox(units);
    typeUnit->addItems({tr("Pixels"), tr("Points"), tr("Millimeters")});
    typeUnit->setCurrentIndex(1);
    typeUnit->setEnabled(false);
    unitsForm->addRow(tr("Rulers:"), rulerUnit_);
    unitsForm->addRow(tr("Type:"), typeUnit);
    layout->addWidget(units, 0, 0);

    const auto valueAndUnit = [](QWidget* parent, const QString& value,
                                 const QStringList& units) {
        auto* row = new QWidget(parent);
        auto* rowLayout = new QHBoxLayout(row);
        rowLayout->setContentsMargins(0, 0, 0, 0);
        rowLayout->addWidget(new QLabel(value, row));
        auto* unit = new QComboBox(row);
        unit->addItems(units);
        rowLayout->addWidget(unit);
        return row;
    };
    const QStringList columnUnits = {tr("Points"), tr("Inches"), tr("Centimeters"),
                                     tr("Millimeters"), tr("Picas")};
    auto* columns = new QGroupBox(tr("Column Size"), page);
    auto* columnsForm = new QFormLayout(columns);
    columnsForm->addRow(tr("Width:"), valueAndUnit(columns, QStringLiteral("180"), columnUnits));
    columnsForm->addRow(tr("Gutter:"), valueAndUnit(columns, QStringLiteral("12"), columnUnits));
    columns->setEnabled(false);
    layout->addWidget(columns, 1, 0);

    const QStringList resolutionUnits = {tr("Pixels/Inch"), tr("Pixels/Centimeter")};
    auto* resolutions = new QGroupBox(tr("New Document Preset Resolutions"), page);
    auto* resolutionsForm = new QFormLayout(resolutions);
    resolutionsForm->addRow(tr("Print Resolution:"),
                            valueAndUnit(resolutions, QStringLiteral("300"), resolutionUnits));
    resolutionsForm->addRow(tr("Screen Resolution:"),
                            valueAndUnit(resolutions, QStringLiteral("72"), resolutionUnits));
    resolutions->setEnabled(false);
    layout->addWidget(resolutions, 0, 1);

    auto* pointSize = new QGroupBox(tr("Point/Pica Size"), page);
    auto* pointLayout = new QVBoxLayout(pointSize);
    postScriptPoints_ = new QRadioButton(tr("PostScript (72 points/inch)"), pointSize);
    postScriptPoints_->setObjectName(QStringLiteral("preferencesPostScriptPoints"));
    traditionalPoints_ = new QRadioButton(tr("Traditional (72.27 points/inch)"), pointSize);
    traditionalPoints_->setObjectName(QStringLiteral("preferencesTraditionalPoints"));
    postScriptPoints_->setChecked(true);
    pointLayout->addWidget(postScriptPoints_);
    pointLayout->addWidget(traditionalPoints_);
    layout->addWidget(pointSize, 1, 1);
    layout->setRowStretch(2, 1);

    connect(rulerUnit_, &QComboBox::activated, this,
            [this](int index) { emit rulerUnitChanged(index); });
    connect(traditionalPoints_, &QRadioButton::toggled, this,
            [this](bool on) { emit traditionalPointsChanged(on); });
    return page;
}

void PreferencesDialog::setRulerUnit(int unit)
{
    const QSignalBlocker block(rulerUnit_);
    rulerUnit_->setCurrentIndex(unit);
}

void PreferencesDialog::setTraditionalPoints(bool on)
{
    const QSignalBlocker blockTraditional(traditionalPoints_);
    const QSignalBlocker blockPostScript(postScriptPoints_);
    (on ? traditionalPoints_ : postScriptPoints_)->setChecked(true);
}

// Guides, Grid, & Slices: the Guides group is live; Smart Guides, Grid, and
// Slices show CS6's defaults, disabled until those features exist.
QWidget* PreferencesDialog::buildGuidesPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("preferencesGuidesPage"));
    auto* layout = new QVBoxLayout(page);

    auto* guides = new QGroupBox(tr("Guides"), page);
    auto* guidesGrid = new QGridLayout(guides);
    guideColor_ = colorCombo(QStringLiteral("preferencesGuideColor"), guides);
    guideStyle_ = new QComboBox(guides);
    guideStyle_->setObjectName(QStringLiteral("preferencesGuideStyle"));
    guideStyle_->addItems({tr("Lines"), tr("Dashed Lines")});
    guideSwatch_ = swatch(QColor(), guides);
    guidesGrid->addWidget(new QLabel(tr("Color:"), guides), 0, 0, Qt::AlignRight);
    guidesGrid->addWidget(guideColor_, 0, 1);
    guidesGrid->addWidget(new QLabel(tr("Style:"), guides), 1, 0, Qt::AlignRight);
    guidesGrid->addWidget(guideStyle_, 1, 1);
    guidesGrid->setColumnStretch(2, 1);
    guidesGrid->addWidget(guideSwatch_, 0, 3, 2, 1);
    layout->addWidget(guides);

    auto* smart = new QGroupBox(tr("Smart Guides"), page);
    auto* smartGrid = new QGridLayout(smart);
    auto* smartColor = colorCombo(QStringLiteral("preferencesSmartGuideColor"), smart);
    smartColor->setCurrentText(QStringLiteral("Magenta"));
    smartGrid->addWidget(new QLabel(tr("Color:"), smart), 0, 0, Qt::AlignRight);
    smartGrid->addWidget(smartColor, 0, 1);
    smartGrid->setColumnStretch(2, 1);
    smartGrid->addWidget(swatch(QColor(255, 74, 255), smart), 0, 3);
    smart->setEnabled(false);
    layout->addWidget(smart);

    auto* grid = new QGroupBox(tr("Grid"), page);
    auto* gridGrid = new QGridLayout(grid);
    auto* gridColor = colorCombo(QStringLiteral("preferencesGridColor"), grid);
    gridColor->setCurrentIndex(gridColor->count() - 1);
    auto* gridStyle = new QComboBox(grid);
    gridStyle->addItems({tr("Lines"), tr("Dashed Lines"), tr("Dots")});
    auto* every = new QComboBox(grid);
    every->addItems({tr("Pixels"), tr("Inches"), tr("Centimeters"), tr("Millimeters"),
                     tr("Points"), tr("Picas"), tr("Percent")});
    every->setCurrentIndex(1);
    auto* everyRow = new QHBoxLayout;
    auto* everyValue = new QLabel(QStringLiteral("1"), grid);
    everyRow->addWidget(everyValue);
    everyRow->addWidget(every);
    gridGrid->addWidget(new QLabel(tr("Color:"), grid), 0, 0, Qt::AlignRight);
    gridGrid->addWidget(gridColor, 0, 1);
    gridGrid->addWidget(new QLabel(tr("Style:"), grid), 1, 0, Qt::AlignRight);
    gridGrid->addWidget(gridStyle, 1, 1);
    gridGrid->addWidget(new QLabel(tr("Gridline Every:"), grid), 2, 0, Qt::AlignRight);
    gridGrid->addLayout(everyRow, 2, 1);
    gridGrid->addWidget(new QLabel(tr("Subdivisions:"), grid), 3, 0, Qt::AlignRight);
    gridGrid->addWidget(new QLabel(QStringLiteral("4"), grid), 3, 1);
    gridGrid->setColumnStretch(2, 1);
    gridGrid->addWidget(swatch(QColor(128, 128, 128), grid), 0, 3, 2, 1);
    grid->setEnabled(false);
    layout->addWidget(grid);

    auto* slices = new QGroupBox(tr("Slices"), page);
    auto* slicesRow = new QHBoxLayout(slices);
    auto* sliceColor = colorCombo(QStringLiteral("preferencesSliceColor"), slices);
    auto* sliceNumbers = new QCheckBox(tr("Show Slice Numbers"), slices);
    sliceNumbers->setChecked(true);
    slicesRow->addWidget(new QLabel(tr("Line Color:"), slices));
    slicesRow->addWidget(sliceColor);
    slicesRow->addWidget(sliceNumbers);
    slicesRow->addStretch(1);
    slices->setEnabled(false);
    layout->addWidget(slices);
    layout->addStretch(1);

    setGuideAppearance(QColor(74, 255, 255), false);
    connect(guideColor_, &QComboBox::activated, this, [this](int index) {
        QColor color = guideColor_->itemData(index).value<QColor>();
        if (!color.isValid()) {
            color = QColorDialog::getColor(guideColorValue_, this, tr("Custom Guide Color"));
            if (!color.isValid()) {
                showGuideColor(guideColorValue_);
                return;
            }
        }
        showGuideColor(color);
        emit guideAppearanceChanged(color, guideStyle_->currentIndex() == 1);
    });
    connect(guideStyle_, &QComboBox::activated, this, [this](int index) {
        emit guideAppearanceChanged(guideColorValue_, index == 1);
    });
    return page;
}

void PreferencesDialog::showGuideColor(const QColor& color)
{
    guideColorValue_ = color;
    int index = guideColor_->count() - 1;
    for (int i = 0; i < guideColor_->count() - 1; ++i) {
        if (guideColor_->itemData(i).value<QColor>() == color) {
            index = i;
            break;
        }
    }
    const QSignalBlocker block(guideColor_);
    guideColor_->setCurrentIndex(index);
    guideSwatch_->setStyleSheet(QStringLiteral("background: %1; border: 1px solid #808080;")
                                    .arg(color.name()));
}

void PreferencesDialog::setGuideAppearance(const QColor& color, bool dashed)
{
    showGuideColor(color);
    const QSignalBlocker block(guideStyle_);
    guideStyle_->setCurrentIndex(dashed ? 1 : 0);
}

} // namespace pictura
