#include "layers_filter_bar.h"

#include "icons.h"
#include "layers_panel_internal.h"

#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

#include <QtCore/QSize>
#include <QtCore/QVariant>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QSizePolicy>

#include <iterator>

namespace pictura {

namespace {

struct Dimension {
    const char* key;
    const char* name;
};

constexpr Dimension kDimensions[] = {
    {"name", "Name"},
    {"kind", "Kind"},
    {"effect", "Effect"},
    {"mode", "Mode"},
    {"attribute", "Attribute"},
    {"color", "Color"},
    {"selected", "Selected"},
};

struct KindOption {
    const char* key;
    const char* name;
};

constexpr KindOption kKinds[] = {
    {"pixel", "Pixel"},
    {"adjustment", "Adjustment"},
    {"group", "Group"},
    {"background", "Background"},
    {"type", "Type"},
};

struct AttributeOption {
    const char* key;
    const char* name;
};

constexpr AttributeOption kAttributes[] = {
    {"visible", "Visible"},
    {"hidden", "Hidden"},
    {"locked", "Locked"},
    {"mask", "Has Mask"},
    {"clipped", "Clipped"},
};

struct ColorOption {
    int label;
    const char* name;
};

constexpr ColorOption kColors[] = {
    {-1, "None"},
    {1, "Red"},
    {2, "Orange"},
    {3, "Yellow"},
    {4, "Green"},
    {5, "Blue"},
    {6, "Violet"},
    {7, "Gray"},
};

QWidget* makePage(QWidget* parent)
{
    return new QWidget(parent);
}

// "dropShadow" -> "Drop Shadow", so the combo shows the CS6 name while the
// item data keeps the bridge key the proxy matches. `bevel` is the one key whose
// CS6 name is not the split identifier ("Bevel & Emboss").
QString effectLabel(const QString& key)
{
    if (key == QLatin1String("bevel")) {
        return QStringLiteral("Bevel & Emboss");
    }
    QString label;
    for (const QChar c : key) {
        if (c.isUpper() && !label.isEmpty()) {
            label += QLatin1Char(' ');
        }
        label += c;
    }
    if (!label.isEmpty()) {
        label[0] = label[0].toUpper();
    }
    return label;
}

} // namespace

LayerFilterBar::LayerFilterBar(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("layersFilterBar"));
    // The criteria pages carry long combo entries; ignoring the size hint keeps
    // the row from forcing a wide minimum on the panel.
    setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Fixed);

    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(2, 1, 2, 1);
    layout->setSpacing(3);

    dimension_ = new QComboBox(this);
    dimension_->setObjectName(QStringLiteral("layersFilterDimension"));
    for (const Dimension& entry : kDimensions) {
        dimension_->addItem(tr(entry.name), QString::fromLatin1(entry.key));
    }
    dimension_->setCurrentIndex(dimension_->findData(QStringLiteral("kind")));
    layout->addWidget(dimension_);

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("layersFilterCriteria"));
    stack_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Fixed);

    auto* namePage = makePage(stack_);
    auto* nameLayout = new QHBoxLayout(namePage);
    nameLayout->setContentsMargins(0, 0, 0, 0);
    name_ = new QLineEdit(namePage);
    name_->setPlaceholderText(tr("Name"));
    nameLayout->addWidget(name_);
    stack_->addWidget(namePage);

    auto* kindPage = makePage(stack_);
    auto* kindLayout = new QHBoxLayout(kindPage);
    kindLayout->setContentsMargins(0, 0, 0, 0);
    kindLayout->setSpacing(2);
    const auto kindAsset = [](const QString& key) {
        if (key == QLatin1String("pixel")) {
            return QStringLiteral("layers.kindPixel");
        }
        if (key == QLatin1String("adjustment")) {
            return QStringLiteral("layers.kindAdjustment");
        }
        if (key == QLatin1String("group")) {
            return QStringLiteral("layers.kindGroup");
        }
        if (key == QLatin1String("type")) {
            return QStringLiteral("layers.kindType");
        }
        return QStringLiteral("layers.kindBackground");
    };
    for (size_t i = 0; i < std::size(kKinds); ++i) {
        auto* button = new QToolButton(kindPage);
        button->setCheckable(true);
        button->setAutoRaise(true);
        const QString key = QString::fromLatin1(kKinds[i].key);
        button->setProperty("kindKey", key);
        button->setToolButtonStyle(Qt::ToolButtonIconOnly);
        button->setIconSize(QSize(16, 16));
        button->setIcon(pictura::icon(kindAsset(key)));
        button->setToolTip(tr(kKinds[i].name));
        kindLayout->addWidget(button);
        kindButtons_[i] = button;
    }
    kindLayout->addStretch(1);
    stack_->addWidget(kindPage);

    auto* effectPage = makePage(stack_);
    auto* effectLayout = new QHBoxLayout(effectPage);
    effectLayout->setContentsMargins(0, 0, 0, 0);
    effect_ = new QComboBox(effectPage);
    effect_->setObjectName(QStringLiteral("layersFilterEffect"));
    for (const QString& key : layer_style_effect_names()) {
        effect_->addItem(effectLabel(key), key);
    }
    effectLayout->addWidget(effect_);
    stack_->addWidget(effectPage);

    auto* modePage = makePage(stack_);
    auto* modeLayout = new QHBoxLayout(modePage);
    modeLayout->setContentsMargins(0, 0, 0, 0);
    mode_ = new QComboBox(modePage);
    for (const BlendEntry& entry : kBlends) {
        mode_->addItem(QString::fromLatin1(entry.name), QString::fromLatin1(entry.key));
    }
    modeLayout->addWidget(mode_);
    stack_->addWidget(modePage);

    auto* attributePage = makePage(stack_);
    auto* attributeLayout = new QHBoxLayout(attributePage);
    attributeLayout->setContentsMargins(0, 0, 0, 0);
    attribute_ = new QComboBox(attributePage);
    for (const AttributeOption& entry : kAttributes) {
        attribute_->addItem(tr(entry.name), QString::fromLatin1(entry.key));
    }
    attributeLayout->addWidget(attribute_);
    stack_->addWidget(attributePage);

    auto* colorPage = makePage(stack_);
    auto* colorLayout = new QHBoxLayout(colorPage);
    colorLayout->setContentsMargins(0, 0, 0, 0);
    color_ = new QComboBox(colorPage);
    for (const ColorOption& entry : kColors) {
        color_->addItem(tr(entry.name), entry.label);
    }
    colorLayout->addWidget(color_);
    stack_->addWidget(colorPage);

    // The Selected dimension has no criteria control: its criterion is the
    // panel's current selection, so its page is empty.
    stack_->addWidget(makePage(stack_));

    stack_->setCurrentIndex(dimension_->currentIndex());
    layout->addWidget(stack_, 1);

    toggle_ = new QToolButton(this);
    toggle_->setObjectName(QStringLiteral("layersFilterToggle"));
    toggle_->setCheckable(true);
    toggle_->setChecked(true);
    toggle_->setAutoRaise(true);
    toggle_->setIconSize(QSize(18, 18));
    toggle_->setIcon(pictura::icon(QStringLiteral("layers.filterOn")));
    toggle_->setToolTip(tr("Enable Layer Filter"));
    layout->addWidget(toggle_);

    connect(dimension_, &QComboBox::currentIndexChanged, this, [this](int index) {
        stack_->setCurrentIndex(index);
        userChanged();
    });
    connect(toggle_, &QToolButton::toggled, this, [this](bool on) {
        toggle_->setIcon(pictura::icon(on ? QStringLiteral("layers.filterOn")
                                          : QStringLiteral("layers.filterOff")));
        userChanged();
    });
    connect(name_, &QLineEdit::textChanged, this, [this](const QString&) { userChanged(); });
    for (QToolButton* button : kindButtons_) {
        connect(button, &QToolButton::toggled, this, [this](bool) { userChanged(); });
    }
    connect(mode_, &QComboBox::currentIndexChanged, this, [this](int) { userChanged(); });
    connect(attribute_, &QComboBox::currentIndexChanged, this, [this](int) { userChanged(); });
    connect(color_, &QComboBox::currentIndexChanged, this, [this](int) { userChanged(); });
    connect(effect_, &QComboBox::currentIndexChanged, this, [this](int) { userChanged(); });
}

QString LayerFilterBar::activeDimension() const
{
    return dimension_->currentData().toString();
}

LayerFilter LayerFilterBar::buildFilter() const
{
    LayerFilter filter;
    filter.enabled = toggle_->isChecked();
    const QString dimension = activeDimension();
    if (dimension == QLatin1String("name")) {
        filter.name = name_->text();
    } else if (dimension == QLatin1String("kind")) {
        for (QToolButton* button : kindButtons_) {
            if (button->isChecked()) {
                filter.kinds.insert(button->property("kindKey").toString());
            }
        }
    } else if (dimension == QLatin1String("mode")) {
        filter.mode = mode_->currentData().toString();
    } else if (dimension == QLatin1String("attribute")) {
        filter.attribute = attribute_->currentData().toString();
    } else if (dimension == QLatin1String("color")) {
        filter.color = color_->currentData().toInt();
    } else if (dimension == QLatin1String("effect")) {
        filter.effect = effect_->currentData().toString();
    } else if (dimension == QLatin1String("selected")) {
        filter.selected = true;
    }
    return filter;
}

void LayerFilterBar::userChanged()
{
    if (syncing_) {
        return;
    }
    filter_ = buildFilter();
    emit filterChanged(filter_);
}

void LayerFilterBar::setFilter(const LayerFilter& filter)
{
    syncing_ = true;
    filter_ = filter;
    toggle_->setChecked(filter.enabled);
    name_->setText(filter.name);
    for (QToolButton* button : kindButtons_) {
        button->setChecked(filter.kinds.contains(button->property("kindKey").toString()));
    }
    mode_->setCurrentIndex(qMax(0, mode_->findData(filter.mode)));
    attribute_->setCurrentIndex(qMax(0, attribute_->findData(filter.attribute)));
    color_->setCurrentIndex(qMax(0, color_->findData(filter.color)));
    effect_->setCurrentIndex(qMax(0, effect_->findData(filter.effect)));

    int dimension = dimension_->findData(QStringLiteral("kind"));
    if (filter.enabled) {
        if (!filter.name.isEmpty()) {
            dimension = dimension_->findData(QStringLiteral("name"));
        } else if (!filter.mode.isEmpty()) {
            dimension = dimension_->findData(QStringLiteral("mode"));
        } else if (!filter.attribute.isEmpty()) {
            dimension = dimension_->findData(QStringLiteral("attribute"));
        } else if (filter.color >= 0) {
            dimension = dimension_->findData(QStringLiteral("color"));
        } else if (!filter.effect.isEmpty()) {
            dimension = dimension_->findData(QStringLiteral("effect"));
        } else if (filter.selected) {
            dimension = dimension_->findData(QStringLiteral("selected"));
        }
    }
    if (dimension >= 0) {
        dimension_->setCurrentIndex(dimension);
        stack_->setCurrentIndex(dimension);
    }
    syncing_ = false;
}

void LayerFilterBar::activateSelected()
{
    const int index = dimension_->findData(QStringLiteral("selected"));
    if (index < 0) {
        return;
    }
    syncing_ = true;
    dimension_->setCurrentIndex(index);
    stack_->setCurrentIndex(index);
    toggle_->setChecked(true);
    syncing_ = false;
    userChanged();
}

void LayerFilterBar::setFilterEnabled(bool on)
{
    if (toggle_->isChecked() == on) {
        return;
    }
    syncing_ = true;
    toggle_->setChecked(on);
    syncing_ = false;
    userChanged();
}

int LayerFilterBar::dimensionIndexForTest() const
{
    return dimension_->currentIndex();
}

bool LayerFilterBar::toggleOnForTest() const
{
    return toggle_->isChecked();
}

bool LayerFilterBar::toggleHasIconForTest() const
{
    return !toggle_->icon().isNull();
}

void LayerFilterBar::setDimensionForTest(const QString& key)
{
    const int index = dimension_->findData(key);
    if (index < 0) {
        return;
    }
    syncing_ = true;
    dimension_->setCurrentIndex(index);
    stack_->setCurrentIndex(index);
    syncing_ = false;
}

} // namespace pictura
