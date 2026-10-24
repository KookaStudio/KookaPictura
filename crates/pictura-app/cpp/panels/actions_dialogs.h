#pragma once

#include <QtCore/QList>
#include <QtCore/QPair>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QDialogButtonBox;
class QDoubleSpinBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QRadioButton;

namespace pictura {

struct ActionDialogSet {
    QString id;
    QString name;
};

class NewActionDialog : public QDialog {
    Q_OBJECT

public:
    explicit NewActionDialog(QWidget* parent = nullptr);

    void setSets(const QList<ActionDialogSet>& sets, const QString& currentSetId = QString());
    QString actionName() const;
    QString setId() const;
    int functionKey() const;
    bool shift() const;
    bool ctrl() const;
    int color() const;
    bool record() const;
    bool validate(QString* error) const;
    QString lastError() const { return lastError_; }

    void accept() override;

private:
    QLineEdit* name_ = nullptr;
    QComboBox* set_ = nullptr;
    QComboBox* key_ = nullptr;
    QCheckBox* shift_ = nullptr;
    QCheckBox* ctrl_ = nullptr;
    QComboBox* color_ = nullptr;
    QCheckBox* record_ = nullptr;
    QLabel* error_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
    QString lastError_;
};

class ActionOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit ActionOptionsDialog(QWidget* parent = nullptr);

    void setSets(const QList<ActionDialogSet>& sets, const QString& currentSetId = QString());
    void setValues(const QString& name, int functionKey, bool shift, bool ctrl, int color);
    QString actionName() const;
    QString setId() const;
    int functionKey() const;
    bool shift() const;
    bool ctrl() const;
    int color() const;
    bool validate(QString* error) const;
    QString lastError() const { return lastError_; }

    void accept() override;

private:
    QLineEdit* name_ = nullptr;
    QComboBox* set_ = nullptr;
    QComboBox* key_ = nullptr;
    QCheckBox* shift_ = nullptr;
    QCheckBox* ctrl_ = nullptr;
    QComboBox* color_ = nullptr;
    QLabel* error_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
    QString lastError_;
};

class SetNameDialog : public QDialog {
    Q_OBJECT

public:
    explicit SetNameDialog(const QString& title, const QString& initialName,
                           QWidget* parent = nullptr);

    QString name() const;
    bool validate(QString* error) const;
    QString lastError() const { return lastError_; }

    void accept() override;

private:
    QLineEdit* name_ = nullptr;
    QLabel* error_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
    QString lastError_;
};

class PlaybackOptionsDialog : public QDialog {
    Q_OBJECT

public:
    enum Mode { Accelerated = 0, StepByStep = 1, Pause = 2 };

    explicit PlaybackOptionsDialog(QWidget* parent = nullptr);

    void setOptions(int mode, double pauseSeconds);
    int mode() const;
    double pauseSeconds() const;

    QRadioButton* acceleratedForTest() const { return accelerated_; }
    QRadioButton* stepByStepForTest() const { return stepByStep_; }
    QRadioButton* pauseForTest() const { return pause_; }
    QDoubleSpinBox* pauseSecondsForTest() const { return seconds_; }

private:
    QRadioButton* accelerated_ = nullptr;
    QRadioButton* stepByStep_ = nullptr;
    QRadioButton* pause_ = nullptr;
    QDoubleSpinBox* seconds_ = nullptr;
};

class InsertStopDialog : public QDialog {
    Q_OBJECT

public:
    explicit InsertStopDialog(QWidget* parent = nullptr);

    QString message() const;
    bool allowContinue() const;
    QPlainTextEdit* messageForTest() const { return message_; }
    QCheckBox* allowContinueForTest() const { return allow_; }

private:
    QPlainTextEdit* message_ = nullptr;
    QCheckBox* allow_ = nullptr;
};

class InsertMenuItemDialog : public QDialog {
    Q_OBJECT

public:
    explicit InsertMenuItemDialog(QWidget* parent = nullptr);

    void setCommands(const QList<QPair<QString, QString>>& commands);
    QString commandId() const;
    QComboBox* commandForTest() const { return command_; }

private:
    QComboBox* command_ = nullptr;
};

class InsertPathDialog : public QDialog {
    Q_OBJECT

public:
    explicit InsertPathDialog(const QString& path, QWidget* parent = nullptr);
    QString path() const { return path_; }

private:
    QString path_;
};

class InsertConditionalDialog : public QDialog {
    Q_OBJECT

public:
    explicit InsertConditionalDialog(QWidget* parent = nullptr);

    void setActions(const QList<ActionDialogSet>& actions);
    void setAllowedActionIds(const QStringList& ids);
    QString condition() const;
    QString thenActionId() const;
    QString elseActionId() const;
    bool validate(QString* error) const;
    QString lastError() const { return lastError_; }

    QComboBox* conditionForTest() const { return condition_; }
    QComboBox* modeForTest() const { return mode_; }
    QComboBox* thenForTest() const { return then_; }
    QComboBox* elseForTest() const { return else_; }

    void accept() override;

private:
    QComboBox* condition_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* then_ = nullptr;
    QComboBox* else_ = nullptr;
    QLabel* error_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
    QStringList allowed_;
    QString lastError_;
};

} // namespace pictura
