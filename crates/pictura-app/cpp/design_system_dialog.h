#pragma once

#include <QtCore/QHash>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtWidgets/QDialog>

#include <functional>

class QCheckBox;
class QCloseEvent;
class QComboBox;
class QLabel;
class QLineEdit;
class QShowEvent;
class QStackedWidget;
class QTreeWidget;
class QTreeWidgetItem;

namespace pictura {

// One "story": a group, a title/blurb, the token keys it exercises, the global
// QSS selector it documents (empty when it maps to none), and a factory for the
// widget page. Pages are built lazily and are ordinary widgets, so what the
// dialog shows is the real UI -- real QSS, Fusion, and custom paint -- not a
// reimplementation.
struct DesignSystemStory {
    QString group;
    QString title;
    QString blurb;
    QStringList tokens;
    QString primitive;
    std::function<QWidget*()> build;
};

// The full story registry (design_system_dialog_pages.cpp).
QVector<DesignSystemStory> designSystemStories();

// Developer > Design System...: an in-app storybook over the real widgets. A
// modeless tool window. The brightness control calls Theme::apply live, so the
// whole application recolours; the level in effect when the dialog opened is
// restored when it closes, so previewing never changes the user's preference.
class DesignSystemDialog : public QDialog {
    Q_OBJECT

public:
    explicit DesignSystemDialog(QWidget* parent = nullptr);

    // Test hooks.
    int storyCountForTest() const { return stories_.size(); }
    QStringList groupsForTest() const;
    QString currentStoryTitleForTest() const;
    int currentLevelForTest() const;
    int currentFamilyForTest() const;
    bool selectStoryForTest(const QString& title);
    void setLevelForTest(int level);
    bool setFilterForTest(const QString& text);
    int visibleStoryCountForTest() const;
    void buildAllPagesForTest();
    int builtPageCountForTest() const { return builtPages_.size(); }

protected:
    void showEvent(QShowEvent* event) override;
    void closeEvent(QCloseEvent* event) override;

private:
    void buildChrome();
    void populateStories();
    void applyFilter(const QString& text);
    void showStory(int index);
    QWidget* pageForStory(int index);
    void refreshTokenPanel();
    void applyStatesVisibility();
    void rebuildThemePages();

    QVector<DesignSystemStory> stories_;
    QHash<int, QWidget*> builtPages_;             // story index -> built page
    QHash<QString, QTreeWidgetItem*> itemByTitle_;
    int currentIndex_ = -1;
    int restoreLevel_ = 0;
    int restoreFamily_ = 0;

    QLineEdit* search_ = nullptr;
    QTreeWidget* tree_ = nullptr;
    QLabel* title_ = nullptr;
    QLabel* blurb_ = nullptr;
    QLabel* primitive_ = nullptr;
    QStackedWidget* pageStack_ = nullptr;
    QComboBox* brightness_ = nullptr;
    QComboBox* family_ = nullptr;
    QCheckBox* showStates_ = nullptr;
    QTreeWidget* tokenTable_ = nullptr;
};

} // namespace pictura
