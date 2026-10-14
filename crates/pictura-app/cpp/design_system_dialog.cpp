#include "design_system_dialog.h"

#include "theme.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QCloseEvent>
#include <QtGui/QFont>
#include <QtGui/QShowEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTreeWidget>
#include <QtWidgets/QTreeWidgetItem>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {
namespace {

constexpr int kStoryIndexRole = Qt::UserRole + 1;

Theme::ThemeFamily familyOf(int family)
{
    return family == 1 ? Theme::ThemeFamily::Light : Theme::ThemeFamily::Dark;
}

QString tokenValueFor(int family, int level, const QString& key)
{
    for (const ThemeToken& token : Theme::tokens(familyOf(family), level)) {
        if (token.key == key) {
            return token.value;
        }
    }
    return {};
}

QLabel* boldLabel(const QString& text, QWidget* parent)
{
    auto* label = new QLabel(text, parent);
    label->setObjectName(QStringLiteral("sectionTitle"));
    return label;
}

} // namespace

DesignSystemDialog::DesignSystemDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("designSystemDialog"));
    setWindowTitle(tr("Design System"));
    setWindowFlag(Qt::Tool, true);
    setAttribute(Qt::WA_DeleteOnClose, false);
    resize(1040, 720);

    restoreLevel_ = Theme::currentLevel();
    restoreFamily_ = Theme::currentFamily() == Theme::ThemeFamily::Light ? 1 : 0;
    stories_ = designSystemStories();
    buildChrome();
    populateStories();
    showStory(0);
}

void DesignSystemDialog::buildChrome()
{
    search_ = new QLineEdit(this);
    search_->setPlaceholderText(tr("Filter stories"));
    search_->setClearButtonEnabled(true);

    tree_ = new QTreeWidget(this);
    tree_->setObjectName(QStringLiteral("designSystemNav"));
    tree_->setHeaderHidden(true);
    tree_->setColumnCount(1);

    auto* left = new QWidget(this);
    auto* leftLayout = new QVBoxLayout(left);
    leftLayout->setContentsMargins(6, 6, 6, 6);
    leftLayout->addWidget(search_);
    leftLayout->addWidget(tree_, 1);

    title_ = boldLabel(QString(), this);
    QFont titleFont = title_->font();
    titleFont.setPointSizeF(titleFont.pointSizeF() * 1.25);
    title_->setFont(titleFont);
    blurb_ = new QLabel(this);
    blurb_->setWordWrap(true);
    primitive_ = new QLabel(this);
    QFont monoFont = primitive_->font();
    monoFont.setFamily(QStringLiteral("monospace"));
    primitive_->setFont(monoFont);

    pageStack_ = new QStackedWidget(this);
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setWidget(pageStack_);

    auto* centre = new QWidget(this);
    auto* centreLayout = new QVBoxLayout(centre);
    centreLayout->addWidget(title_);
    centreLayout->addWidget(blurb_);
    centreLayout->addWidget(primitive_);
    centreLayout->addWidget(scroll, 1);

    brightness_ = new QComboBox(this);
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        if (level == 0) {
            brightness_->addItem(tr("Level 0 (darkest)"));
        } else if (level == Theme::kLevelCount - 1) {
            brightness_->addItem(tr("Level %1 (lightest)").arg(level));
        } else {
            brightness_->addItem(tr("Level %1").arg(level));
        }
    }
    brightness_->setCurrentIndex(Theme::currentLevel());

    showStates_ = new QCheckBox(tr("Show states"), this);
    showStates_->setChecked(true);

    tokenTable_ = new QTreeWidget(this);
    tokenTable_->setObjectName(QStringLiteral("designSystemTokens"));
    tokenTable_->setColumnCount(2);
    tokenTable_->setHeaderLabels({tr("Token"), tr("Value")});
    tokenTable_->setRootIsDecorated(false);

    auto* right = new QWidget(this);
    auto* rightLayout = new QVBoxLayout(right);
    rightLayout->addWidget(boldLabel(tr("Family"), this));
    family_ = new QComboBox(this);
    family_->addItem(tr("Dark"), 0);
    family_->addItem(tr("Light"), 1);
    family_->setCurrentIndex(restoreFamily_);
    rightLayout->addWidget(family_);
    rightLayout->addWidget(boldLabel(tr("Brightness"), this));
    rightLayout->addWidget(brightness_);
    rightLayout->addWidget(showStates_);
    rightLayout->addWidget(boldLabel(tr("Tokens"), this));
    rightLayout->addWidget(tokenTable_, 1);

    auto* splitter = new QSplitter(Qt::Horizontal, this);
    splitter->addWidget(left);
    splitter->addWidget(centre);
    splitter->addWidget(right);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setStretchFactor(2, 0);
    splitter->setSizes({230, 560, 250});

    auto* outer = new QVBoxLayout(this);
    outer->addWidget(splitter);

    connect(search_, &QLineEdit::textChanged, this, &DesignSystemDialog::applyFilter);
    connect(tree_, &QTreeWidget::currentItemChanged, this,
            [this](QTreeWidgetItem* current, QTreeWidgetItem*) {
                if (!current) {
                    return;
                }
                const QVariant data = current->data(0, kStoryIndexRole);
                if (data.isValid()) {
                    showStory(data.toInt());
                }
            });
    // Foundations pages paint token colours directly, so rebuild them for a new
    // family or level instead of relying on palette propagation.
    connect(brightness_, &QComboBox::currentIndexChanged, this,
            [this](int level) {
                Theme::apply(familyOf(currentFamilyForTest()), level);
                rebuildThemePages();
            });
    connect(family_, &QComboBox::currentIndexChanged, this,
            [this](int family) {
                Theme::apply(familyOf(family), Theme::currentLevel());
                rebuildThemePages();
            });
    connect(showStates_, &QCheckBox::toggled, this, [this](bool) { applyStatesVisibility(); });
}

void DesignSystemDialog::populateStories()
{
    QHash<QString, QTreeWidgetItem*> groups;
    for (int i = 0; i < stories_.size(); ++i) {
        const DesignSystemStory& story = stories_.at(i);
        QTreeWidgetItem* group = groups.value(story.group);
        if (!group) {
            group = new QTreeWidgetItem(tree_, QStringList{story.group});
            group->setFlags(Qt::ItemIsEnabled);
            group->setExpanded(true);
            groups.insert(story.group, group);
        }
        auto* item = new QTreeWidgetItem(group, QStringList{story.title});
        item->setData(0, kStoryIndexRole, i);
        itemByTitle_.insert(story.title, item);
    }
}

void DesignSystemDialog::applyFilter(const QString& text)
{
    const QString needle = text.trimmed();
    for (int g = 0; g < tree_->topLevelItemCount(); ++g) {
        QTreeWidgetItem* group = tree_->topLevelItem(g);
        int visible = 0;
        for (int c = 0; c < group->childCount(); ++c) {
            QTreeWidgetItem* child = group->child(c);
            const bool match = needle.isEmpty()
                || child->text(0).contains(needle, Qt::CaseInsensitive);
            child->setHidden(!match);
            visible += match ? 1 : 0;
        }
        group->setHidden(visible == 0);
        if (!needle.isEmpty() && visible > 0) {
            group->setExpanded(true);
        }
    }
}

QWidget* DesignSystemDialog::pageForStory(int index)
{
    if (QWidget* existing = builtPages_.value(index)) {
        return existing;
    }
    const DesignSystemStory& story = stories_.at(index);
    QWidget* page = story.build ? story.build() : new QWidget;
    pageStack_->addWidget(page);
    builtPages_.insert(index, page);
    applyStatesVisibility();
    return page;
}

void DesignSystemDialog::showStory(int index)
{
    if (index < 0 || index >= stories_.size()) {
        return;
    }
    currentIndex_ = index;
    const DesignSystemStory& story = stories_.at(index);
    title_->setText(story.title);
    blurb_->setText(story.blurb);
    primitive_->setText(story.primitive.isEmpty()
                            ? QString()
                            : QStringLiteral("QSS: ") + story.primitive);
    primitive_->setVisible(!story.primitive.isEmpty());
    pageStack_->setCurrentWidget(pageForStory(index));
    if (QTreeWidgetItem* item = itemByTitle_.value(story.title)) {
        const QSignalBlocker blocker(tree_);
        tree_->setCurrentItem(item);
        tree_->scrollToItem(item);
    }
    refreshTokenPanel();
}

void DesignSystemDialog::refreshTokenPanel()
{
    tokenTable_->clear();
    if (currentIndex_ < 0) {
        return;
    }
    const int level = Theme::currentLevel();
    const int family = currentFamilyForTest();
    QStringList keys = stories_.at(currentIndex_).tokens;
    if (keys.isEmpty()) {
        for (const ThemeToken& token : Theme::tokens(familyOf(family), level)) {
            if (token.color) {
                keys.append(token.key);
            }
        }
        keys.sort();
    }
    for (const QString& key : keys) {
        new QTreeWidgetItem(tokenTable_,
                            {QStringLiteral("--") + key, tokenValueFor(family, level, key)});
    }
}

void DesignSystemDialog::applyStatesVisibility()
{
    const bool on = showStates_ == nullptr || showStates_->isChecked();
    for (QWidget* page : builtPages_) {
        const QList<QWidget*> sections =
            page->findChildren<QWidget*>(QStringLiteral("dsStates"));
        for (QWidget* section : sections) {
            section->setVisible(on);
        }
    }
}

void DesignSystemDialog::closeEvent(QCloseEvent* event)
{
    Theme::apply(familyOf(restoreFamily_), restoreLevel_);
    QDialog::closeEvent(event);
}

// The dialog is reused (not recreated) per open, so re-anchor the restore
// snapshot and the controls to the theme in effect now; otherwise closing would
// revert the user's brightness/family to the value captured at first open.
void DesignSystemDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    restoreLevel_ = Theme::currentLevel();
    restoreFamily_ = Theme::currentFamily() == Theme::ThemeFamily::Light ? 1 : 0;
    {
        const QSignalBlocker blockBrightness(brightness_);
        const QSignalBlocker blockFamily(family_);
        brightness_->setCurrentIndex(restoreLevel_);
        family_->setCurrentIndex(restoreFamily_);
    }
    rebuildThemePages();
}

void DesignSystemDialog::rebuildThemePages()
{
    for (QWidget* page : builtPages_) {
        pageStack_->removeWidget(page);
        page->deleteLater();
    }
    builtPages_.clear();
    showStory(currentIndex_);
}

int DesignSystemDialog::currentLevelForTest() const
{
    return brightness_ ? brightness_->currentIndex() : -1;
}

int DesignSystemDialog::currentFamilyForTest() const
{
    return family_ ? family_->currentIndex() : restoreFamily_;
}

QStringList DesignSystemDialog::groupsForTest() const
{
    QStringList groups;
    for (const DesignSystemStory& story : stories_) {
        if (!groups.contains(story.group)) {
            groups.append(story.group);
        }
    }
    return groups;
}

QString DesignSystemDialog::currentStoryTitleForTest() const
{
    return title_ ? title_->text() : QString();
}

bool DesignSystemDialog::selectStoryForTest(const QString& title)
{
    QTreeWidgetItem* item = itemByTitle_.value(title);
    if (!item) {
        return false;
    }
    tree_->setCurrentItem(item);
    showStory(item->data(0, kStoryIndexRole).toInt());
    return true;
}

void DesignSystemDialog::setLevelForTest(int level)
{
    if (brightness_) {
        brightness_->setCurrentIndex(level);
    }
}

bool DesignSystemDialog::setFilterForTest(const QString& text)
{
    if (!search_) {
        return false;
    }
    search_->setText(text);
    return true;
}

int DesignSystemDialog::visibleStoryCountForTest() const
{
    int count = 0;
    for (int g = 0; g < tree_->topLevelItemCount(); ++g) {
        QTreeWidgetItem* group = tree_->topLevelItem(g);
        if (group->isHidden()) {
            continue;
        }
        for (int c = 0; c < group->childCount(); ++c) {
            if (!group->child(c)->isHidden()) {
                ++count;
            }
        }
    }
    return count;
}

void DesignSystemDialog::buildAllPagesForTest()
{
    for (int i = 0; i < stories_.size(); ++i) {
        pageForStory(i);
    }
}

} // namespace pictura
