#include "fill_dialog.h"
#include <QtCore/QCoreApplication>

#include "color_picker_dialog.h"
#include "options_bar.h"

#include "api/paint_bridge.h"

#include <QtGui/QPixmap>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {
namespace {

enum class FillContent {
    None, Foreground, Background, Custom, ContentAware, Pattern, History, Black, Gray, White,
};

} // namespace

FillDialog::FillDialog(const QColor& foreground, const QColor& background, QWidget* parent)
    : QDialog(parent)
    , foreground_(foreground)
    , background_(background)
{
    setWindowTitle(tr("Fill"));
    setMinimumSize(400, 330);

    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    auto* contentsRow = new QHBoxLayout;
    auto* contentsLabel = new QLabel(tr("Contents:"), this);
    contentsLabel->setFixedWidth(65);
    contentsLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    contentsRow->addWidget(contentsLabel);
    contents_ = new QComboBox(this);
    contents_->setObjectName(QStringLiteral("fillContents"));
    contents_->addItem(tr("Foreground Color"), int(FillContent::Foreground));
    contents_->addItem(tr("Background Color"), int(FillContent::Background));
    contents_->addItem(tr("Color..."), int(FillContent::Custom));
    contents_->insertSeparator(3);
    contents_->addItem(tr("Content-Aware"), int(FillContent::ContentAware));
    contents_->addItem(tr("Pattern"), int(FillContent::Pattern));
    contents_->addItem(tr("History"), int(FillContent::History));
    contents_->insertSeparator(7);
    contents_->addItem(tr("Black"), int(FillContent::Black));
    contents_->addItem(tr("50% Gray"), int(FillContent::Gray));
    contents_->addItem(tr("White"), int(FillContent::White));
    contents_->setMinimumWidth(160);
    if (auto* model = qobject_cast<QStandardItemModel*>(contents_->model())) {
        for (int i : {4, 6}) {
            if (auto* item = model->item(i)) {
                item->setEnabled(false);
            }
        }
    }
    contentsRow->addWidget(contents_, 1);
    left->addLayout(contentsRow);
    left->addSpacing(6);

    auto* patternGroup = new QGroupBox(tr("Options"), this);
    auto* patternLayout = new QHBoxLayout(patternGroup);
    patternLayout->addWidget(new QLabel(tr("Custom Pattern:"), patternGroup));
    patternSwatch_ = new QToolButton(patternGroup);
    patternSwatch_->setObjectName(QStringLiteral("fillPattern"));
    patternSwatch_->setIconSize(QSize(32, 32));
    patternSwatch_->setFixedSize(40, 40);
    patternSwatch_->setPopupMode(QToolButton::InstantPopup);
    patternLayout->addWidget(patternSwatch_);
    patternLayout->addStretch();
    left->addWidget(patternGroup);
    patternGroup->setVisible(false);
    contents_->setProperty("patternGroup", QVariant::fromValue<QObject*>(patternGroup));

    scripted_ = new QCheckBox(tr("Scripted Patterns"), patternGroup);
    scripted_->setObjectName(QStringLiteral("fillScriptedPatterns"));
    script_ = new QComboBox(patternGroup);
    script_->setObjectName(QStringLiteral("fillScript"));
    script_->addItem(tr("Brick Fill"), 1);
    script_->addItem(tr("Cross Weave"), 2);
    script_->addItem(tr("Random Fill"), 3);
    script_->addItem(tr("Spiral"), 4);
    script_->addItem(tr("Symmetry Fill"), 5);
    script_->setEnabled(false);
    connect(scripted_, &QCheckBox::toggled, script_, &QComboBox::setEnabled);
    left->addWidget(scripted_);
    left->addWidget(script_);
    scripted_->setVisible(false);
    script_->setVisible(false);
    patterns_ = PresetStore::current().rows(PresetType::Patterns);
    if (patterns_.isEmpty()) {
        if (auto* model = qobject_cast<QStandardItemModel*>(contents_->model())) {
            if (auto* item = model->item(contents_->findData(int(FillContent::Pattern)))) item->setEnabled(false);
        }
    }
    buildPatternGrid();

    auto* blendGroup = new QGroupBox(tr("Blending"), this);
    auto* blendLayout = new QVBoxLayout(blendGroup);
    auto* modeRow = new QHBoxLayout;
    auto* modeLabel = new QLabel(tr("Mode:"), blendGroup);
    modeLabel->setFixedWidth(55);
    modeLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    modeRow->addWidget(modeLabel);
    mode_ = new QComboBox(blendGroup);
    mode_->setObjectName(QStringLiteral("fillMode"));
    mode_->addItem(QCoreApplication::translate("PicturaUi", "Normal"), QStringLiteral("normal"));
    mode_->addItem(QCoreApplication::translate("PicturaUi", "Dissolve"), QStringLiteral("dissolve"));
    mode_->addItem(QCoreApplication::translate("PicturaUi", "Behind"), QStringLiteral("behind"));
    mode_->addItem(QCoreApplication::translate("PicturaUi", "Clear"), QStringLiteral("clear"));
    mode_->setMinimumWidth(140);
    modeRow->addWidget(mode_, 1);
    blendLayout->addLayout(modeRow);

    auto* opacityRow = new QHBoxLayout;
    auto* opacityLabel = new QLabel(tr("Opacity:"), blendGroup);
    opacityLabel->setFixedWidth(55);
    opacityLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    opacityRow->addWidget(opacityLabel);
    opacity_ = new QSpinBox(blendGroup);
    opacity_->setObjectName(QStringLiteral("fillOpacity"));
    opacity_->setRange(1, 100);
    opacity_->setValue(100);
    opacity_->setSuffix(QStringLiteral(" %"));
    opacity_->setFixedWidth(70);
    opacityRow->addWidget(opacity_);
    opacityRow->addStretch();
    blendLayout->addLayout(opacityRow);
    left->addWidget(blendGroup);

    preserveTransparency_ = new QCheckBox(tr("Preserve Transparency"), this);
    preserveTransparency_->setObjectName(QStringLiteral("fillPreserveTransparency"));
    left->addWidget(preserveTransparency_);
    left->addStretch();
    outer->addLayout(left, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(tr("OK"), this);
    ok->setDefault(true);
    ok->setFixedWidth(80);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setFixedWidth(80);
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addStretch();
    outer->addLayout(buttons);

    connect(contents_, QOverload<int>::of(&QComboBox::currentIndexChanged), this,
            &FillDialog::onContentsChanged);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
}

void FillDialog::buildPatternGrid()
{
    patternPopup_ = new QWidget(this, Qt::Popup);
    auto* grid = new QGridLayout(patternPopup_);
    grid->setSpacing(2);
    grid->setContentsMargins(4, 4, 4, 4);

    const int count = patterns_.size();
    const int cols = 7;
    for (int i = 0; i < count; ++i) {
        auto* button = new QToolButton(patternPopup_);
        const auto& row = patterns_.at(i);
        const QIcon icon = row.preview.isNull() ? patternIcon(row.values.value(QStringLiteral("index")).toInt()) : QIcon(QPixmap::fromImage(row.preview));
        button->setIcon(icon);
        button->setIconSize(QSize(32, 32));
        button->setFixedSize(36, 36);
        button->setToolTip(row.name);
        connect(button, &QToolButton::clicked, this, [this, i, icon] {
            selectedPattern_ = i;
            patternSwatch_->setIcon(icon);
            patternPopup_->hide();
        });
        grid->addWidget(button, i / cols, i % cols);
    }
    if (count > 0) {
        const auto& row = patterns_.first();
        patternSwatch_->setIcon(row.preview.isNull() ? patternIcon(row.values.value(QStringLiteral("index")).toInt()) : QIcon(QPixmap::fromImage(row.preview)));
    }

    connect(patternSwatch_, &QToolButton::clicked, this, [this] {
        const QPoint pos = patternSwatch_->mapToGlobal(QPoint(0, patternSwatch_->height()));
        patternPopup_->move(pos);
        patternPopup_->show();
    });
}

void FillDialog::onContentsChanged(int index)
{
    const auto content = static_cast<FillContent>(contents_->itemData(index).toInt());
    if (auto* group = qobject_cast<QGroupBox*>(
            contents_->property("patternGroup").value<QObject*>())) {
        group->setVisible(content == FillContent::Pattern);
    }
    scripted_->setVisible(content == FillContent::Pattern);
    script_->setVisible(content == FillContent::Pattern);
    if (content == FillContent::Custom) {
        chooseColor();
    }
}

void FillDialog::chooseColor()
{
    const QColor picked = ColorPickerDialog::getColor(
        customColor_.isValid() ? customColor_ : QColor(Qt::black), this, tr("Fill Color"));
    if (picked.isValid()) {
        customColor_ = picked;
    } else {
        contents_->setCurrentIndex(0);
    }
}

bool FillDialog::isPatternFill() const
{
    return contents_->currentData().toInt() == int(FillContent::Pattern);
}

int FillDialog::selectedPatternIndex() const { return selectedPattern_; }
QByteArray FillDialog::selectedPatternBytes() const
{
    return selectedPattern_ >= 0 && selectedPattern_ < patterns_.size() ? patterns_.at(selectedPattern_).native : QByteArray();
}
int FillDialog::script() const { return scripted_->isChecked() ? script_->currentData().toInt() : 0; }

QColor FillDialog::fillColor() const
{
    switch (static_cast<FillContent>(contents_->currentData().toInt())) {
    case FillContent::Foreground: return foreground_;
    case FillContent::Background: return background_;
    case FillContent::Custom: return customColor_.isValid() ? customColor_ : QColor(Qt::black);
    case FillContent::Gray: return QColor(128, 128, 128);
    case FillContent::White: return QColor(Qt::white);
    default: return QColor(Qt::black);
    }
}

QString FillDialog::blendMode() const { return mode_->currentData().toString(); }

int FillDialog::opacity() const { return opacity_->value(); }

bool FillDialog::preserveTransparency() const { return preserveTransparency_->isChecked(); }

} // namespace pictura

#include "moc_fill_dialog.cpp"
