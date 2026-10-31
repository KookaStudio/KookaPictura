#pragma once

#include <QtWidgets/QWidget>

class QCheckBox;
class QComboBox;
class QLabel;
class QListWidget;
class QPlainTextEdit;
class QPushButton;

namespace pictura {

class AgentControl;

// Edit > Preferences > Agent Control. Not a CS6 pane. Every change goes straight
// to AgentControl, which persists it, as the other pages' changes do.
class AgentControlPage : public QWidget {
    Q_OBJECT

public:
    explicit AgentControlPage(AgentControl* agent, QWidget* parent = nullptr);

    // Add `path` to the allowed folders; the Add button asks for it.
    void addFolder(const QString& path);

private:
    void applyControls();
    void showSettings();
    void showStatus();

    AgentControl* agent_;
    QCheckBox* enable_;
    QLabel* status_;
    QCheckBox* allowEdit_;
    QCheckBox* allowSave_;
    QComboBox* approval_;
    QLabel* allowAllWarning_;
    QListWidget* folders_;
    QPushButton* addFolder_;
    QPushButton* removeFolder_;
    QPlainTextEdit* snippet_;
    QPushButton* copySnippet_;
};

} // namespace pictura
