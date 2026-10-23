#include "actions_dialogs.h"

#include "actions_tree_model.h"

#include <QtCore/QVariant>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

void populateSets(QComboBox* combo, const QList<ActionDialogSet>& sets,
                  const QString& currentSetId)
{
    combo->clear();
    for (const ActionDialogSet& set : sets) {
        combo->addItem(set.name, set.id);
        if (!currentSetId.isEmpty() && set.id == currentSetId) {
            combo->setCurrentIndex(combo->count() - 1);
        }
    }
}

void populateFunctionKeys(QComboBox* combo)
{
    combo->clear();
    combo->addItem(QObject::tr("None"), 0);
    for (int i = 0; i < 12; ++i) {
        const int key = Qt::Key_F1 + i;
        combo->addItem(QObject::tr("F%1").arg(i + 1), key);
    }
}

void selectFunctionKey(QComboBox* combo, int key)
{
    const int index = combo->findData(key);
    combo->setCurrentIndex(index >= 0 ? index : 0);
}

void populateColors(QComboBox* combo)
{
    combo->clear();
    const QColor colors[] = {
        actionColor(0), actionColor(1), actionColor(2), actionColor(3),
        actionColor(4), actionColor(5), actionColor(6), actionColor(7),
    };
    const QStringList names = actionColorNames();
    for (int i = 0; i < names.size(); ++i) {
        QPixmap swatch(12, 12);
        swatch.fill(colors[i]);
        combo->addItem(QIcon(swatch), names.at(i), i);
    }
}

bool validateNameKey(const QString& name, int key, bool shift, bool ctrl, QString* error)
{
    if (name.trimmed().isEmpty()) {
        if (error) {
            *error = QObject::tr("Action name is required.");
        }
        return false;
    }
    if (key != 0 && (key < Qt::Key_F1 || key > Qt::Key_F12)) {
        if (error) {
            *error = QObject::tr("Choose a valid function key (F1-F12).");
        }
        return false;
    }
    if (key == 0 && (shift || ctrl)) {
        if (error) {
            *error = QObject::tr("Choose a function key for the modifiers.");
        }
        return false;
    }
    return true;
}

QLabel* makeErrorLabel(QWidget* parent)
{
    auto* label = new QLabel(parent);
    label->setObjectName(QStringLiteral("actionsDialogError"));
    label->setStyleSheet(QStringLiteral("color: #e74c3c;"));
    label->setWordWrap(true);
    label->hide();
    return label;
}

QDialogButtonBox* makeButtons(QDialog* dialog)
{
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, dialog, &QDialog::reject);
    return buttons;
}

void showError(QLabel* label, QDialogButtonBox* buttons, const QString& error)
{
    label->setText(error);
    label->setVisible(!error.isEmpty());
    if (!error.isEmpty() && buttons) {
        buttons->button(QDialogButtonBox::Ok)->setEnabled(true);
    }
}

} // namespace

NewActionDialog::NewActionDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("New Action"));
    name_ = new QLineEdit(this);
    name_->setObjectName(QStringLiteral("actionsDialogName"));
    set_ = new QComboBox(this);
    set_->setObjectName(QStringLiteral("actionsDialogSet"));
    key_ = new QComboBox(this);
    key_->setObjectName(QStringLiteral("actionsDialogFunctionKey"));
    populateFunctionKeys(key_);
    shift_ = new QCheckBox(tr("Shift"), this);
    shift_->setObjectName(QStringLiteral("actionsDialogShift"));
    ctrl_ = new QCheckBox(tr("Ctrl"), this);
    ctrl_->setObjectName(QStringLiteral("actionsDialogCtrl"));
    color_ = new QComboBox(this);
    color_->setObjectName(QStringLiteral("actionsDialogColor"));
    populateColors(color_);
    record_ = new QCheckBox(tr("Record"), this);
    record_->setObjectName(QStringLiteral("actionsDialogRecord"));
    record_->setChecked(true);
    error_ = makeErrorLabel(this);
    buttons_ = makeButtons(this);

    auto* modifiers = new QHBoxLayout;
    modifiers->addWidget(shift_);
    modifiers->addWidget(ctrl_);
    modifiers->addStretch();

    auto* form = new QFormLayout;
    form->addRow(tr("Name:"), name_);
    form->addRow(tr("Set:"), set_);
    form->addRow(tr("Function Key:"), key_);
    form->addRow(QString(), modifiers);
    form->addRow(tr("Color:"), color_);
    form->addRow(QString(), record_);

    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(error_);
    layout->addWidget(buttons_);
}

void NewActionDialog::setSets(const QList<ActionDialogSet>& sets, const QString& currentSetId)
{
    populateSets(set_, sets, currentSetId);
}

QString NewActionDialog::actionName() const
{
    return name_->text().trimmed();
}

QString NewActionDialog::setId() const
{
    return set_->currentData().toString();
}

int NewActionDialog::functionKey() const
{
    return key_->currentData().toInt();
}

bool NewActionDialog::shift() const
{
    return shift_->isChecked();
}

bool NewActionDialog::ctrl() const
{
    return ctrl_->isChecked();
}

int NewActionDialog::color() const
{
    return color_->currentData().toInt();
}

bool NewActionDialog::record() const
{
    return record_->isChecked();
}

bool NewActionDialog::validate(QString* error) const
{
    return validateNameKey(actionName(), functionKey(), shift(), ctrl(), error);
}

void NewActionDialog::accept()
{
    if (!validate(&lastError_)) {
        showError(error_, buttons_, lastError_);
        return;
    }
    QDialog::accept();
}

ActionOptionsDialog::ActionOptionsDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Action Options"));
    name_ = new QLineEdit(this);
    name_->setObjectName(QStringLiteral("actionsDialogName"));
    set_ = new QComboBox(this);
    set_->setObjectName(QStringLiteral("actionsDialogSet"));
    key_ = new QComboBox(this);
    key_->setObjectName(QStringLiteral("actionsDialogFunctionKey"));
    populateFunctionKeys(key_);
    shift_ = new QCheckBox(tr("Shift"), this);
    shift_->setObjectName(QStringLiteral("actionsDialogShift"));
    ctrl_ = new QCheckBox(tr("Ctrl"), this);
    ctrl_->setObjectName(QStringLiteral("actionsDialogCtrl"));
    color_ = new QComboBox(this);
    color_->setObjectName(QStringLiteral("actionsDialogColor"));
    populateColors(color_);
    error_ = makeErrorLabel(this);
    buttons_ = makeButtons(this);

    auto* modifiers = new QHBoxLayout;
    modifiers->addWidget(shift_);
    modifiers->addWidget(ctrl_);
    modifiers->addStretch();

    auto* form = new QFormLayout;
    form->addRow(tr("Name:"), name_);
    form->addRow(tr("Set:"), set_);
    form->addRow(tr("Function Key:"), key_);
    form->addRow(QString(), modifiers);
    form->addRow(tr("Color:"), color_);

    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(error_);
    layout->addWidget(buttons_);
}

void ActionOptionsDialog::setSets(const QList<ActionDialogSet>& sets,
                                  const QString& currentSetId)
{
    populateSets(set_, sets, currentSetId);
}

void ActionOptionsDialog::setValues(const QString& name, int functionKey, bool shift, bool ctrl,
                                    int color)
{
    name_->setText(name);
    selectFunctionKey(key_, functionKey);
    shift_->setChecked(shift);
    ctrl_->setChecked(ctrl);
    const int index = color_->findData(color);
    color_->setCurrentIndex(index >= 0 ? index : 0);
}

QString ActionOptionsDialog::actionName() const
{
    return name_->text().trimmed();
}

QString ActionOptionsDialog::setId() const
{
    return set_->currentData().toString();
}

int ActionOptionsDialog::functionKey() const
{
    return key_->currentData().toInt();
}

bool ActionOptionsDialog::shift() const
{
    return shift_->isChecked();
}

bool ActionOptionsDialog::ctrl() const
{
    return ctrl_->isChecked();
}

int ActionOptionsDialog::color() const
{
    return color_->currentData().toInt();
}

bool ActionOptionsDialog::validate(QString* error) const
{
    return validateNameKey(actionName(), functionKey(), shift(), ctrl(), error);
}

void ActionOptionsDialog::accept()
{
    if (!validate(&lastError_)) {
        showError(error_, buttons_, lastError_);
        return;
    }
    QDialog::accept();
}

SetNameDialog::SetNameDialog(const QString& title, const QString& initialName, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(title);
    name_ = new QLineEdit(initialName, this);
    name_->setObjectName(QStringLiteral("actionsDialogName"));
    error_ = makeErrorLabel(this);
    buttons_ = makeButtons(this);

    auto* form = new QFormLayout;
    form->addRow(tr("Name:"), name_);
    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(error_);
    layout->addWidget(buttons_);
}

QString SetNameDialog::name() const
{
    return name_->text().trimmed();
}

bool SetNameDialog::validate(QString* error) const
{
    if (name().isEmpty()) {
        if (error) {
            *error = tr("Set name is required.");
        }
        return false;
    }
    return true;
}

void SetNameDialog::accept()
{
    if (!validate(&lastError_)) {
        showError(error_, buttons_, lastError_);
        return;
    }
    QDialog::accept();
}

PlaybackOptionsDialog::PlaybackOptionsDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Playback Options"));
    accelerated_ = new QRadioButton(tr("Accelerated"), this);
    accelerated_->setObjectName(QStringLiteral("actionsPlaybackAccelerated"));
    stepByStep_ = new QRadioButton(tr("Step By Step"), this);
    stepByStep_->setObjectName(QStringLiteral("actionsPlaybackStep"));
    pause_ = new QRadioButton(tr("Pause For"), this);
    pause_->setObjectName(QStringLiteral("actionsPlaybackPause"));
    seconds_ = new QDoubleSpinBox(this);
    seconds_->setObjectName(QStringLiteral("actionsPlaybackSeconds"));
    seconds_->setRange(0.0, 3600.0);
    seconds_->setDecimals(1);
    seconds_->setSuffix(tr(" seconds"));
    accelerated_->setChecked(true);
    seconds_->setEnabled(false);
    connect(stepByStep_, &QRadioButton::toggled, this, [this](bool on) {
        if (on) {
            seconds_->setEnabled(false);
        }
    });
    connect(pause_, &QRadioButton::toggled, this, [this](bool on) {
        seconds_->setEnabled(on);
    });

    auto* pauseRow = new QHBoxLayout;
    pauseRow->addWidget(pause_);
    pauseRow->addWidget(seconds_);
    pauseRow->addStretch();

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(accelerated_);
    layout->addWidget(stepByStep_);
    layout->addLayout(pauseRow);
    layout->addWidget(makeButtons(this));
}

void PlaybackOptionsDialog::setOptions(int mode, double pauseSeconds)
{
    if (mode == StepByStep) {
        stepByStep_->setChecked(true);
    } else if (mode == Pause) {
        pause_->setChecked(true);
    } else {
        accelerated_->setChecked(true);
    }
    seconds_->setValue(qMax(0.0, pauseSeconds));
}

int PlaybackOptionsDialog::mode() const
{
    if (stepByStep_->isChecked()) {
        return StepByStep;
    }
    if (pause_->isChecked()) {
        return Pause;
    }
    return Accelerated;
}

double PlaybackOptionsDialog::pauseSeconds() const
{
    return seconds_->value();
}

InsertStopDialog::InsertStopDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Insert Stop"));
    message_ = new QPlainTextEdit(this);
    message_->setObjectName(QStringLiteral("actionsStopMessage"));
    allow_ = new QCheckBox(tr("Allow Continue"), this);
    allow_->setObjectName(QStringLiteral("actionsStopAllowContinue"));
    auto* form = new QFormLayout;
    form->addRow(tr("Message:"), message_);
    form->addRow(QString(), allow_);
    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(makeButtons(this));
}

QString InsertStopDialog::message() const
{
    return message_->toPlainText();
}

bool InsertStopDialog::allowContinue() const
{
    return allow_->isChecked();
}

InsertMenuItemDialog::InsertMenuItemDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Insert Menu Item"));
    command_ = new QComboBox(this);
    command_->setObjectName(QStringLiteral("actionsMenuItemCommand"));
    auto* form = new QFormLayout;
    form->addRow(tr("Command:"), command_);
    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(makeButtons(this));
}

void InsertMenuItemDialog::setCommands(const QList<QPair<QString, QString>>& commands)
{
    command_->clear();
    for (const auto& command : commands) {
        command_->addItem(command.second, command.first);
    }
}

QString InsertMenuItemDialog::commandId() const
{
    return command_->currentData().toString();
}

InsertPathDialog::InsertPathDialog(const QString& path, QWidget* parent)
    : QDialog(parent)
    , path_(path)
{
    setWindowTitle(tr("Insert Path"));
    auto* label = new QLabel(path, this);
    label->setObjectName(QStringLiteral("actionsInsertPathName"));
    label->setWordWrap(true);
    auto* form = new QFormLayout;
    form->addRow(tr("Path:"), label);
    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(makeButtons(this));
}

InsertConditionalDialog::InsertConditionalDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Insert Conditional"));
    condition_ = new QComboBox(this);
    condition_->setObjectName(QStringLiteral("actionsConditionalCondition"));
    condition_->addItem(tr("Document Is Square"), QStringLiteral("square"));
    condition_->addItem(tr("Document Mode"), QStringLiteral("mode"));
    condition_->addItem(tr("Document Has A Selection"), QStringLiteral("selection"));
    mode_ = new QComboBox(this);
    mode_->setObjectName(QStringLiteral("actionsConditionalMode"));
    mode_->addItem(tr("Bitmap"), QStringLiteral("bitmap"));
    mode_->addItem(tr("Grayscale"), QStringLiteral("grayscale"));
    mode_->addItem(tr("Indexed Color"), QStringLiteral("indexed"));
    mode_->addItem(tr("RGB Color"), QStringLiteral("rgb"));
    mode_->addItem(tr("CMYK Color"), QStringLiteral("cmyk"));
    mode_->addItem(tr("Lab Color"), QStringLiteral("lab"));
    mode_->setEnabled(false);
    connect(condition_, &QComboBox::currentIndexChanged, this, [this] {
        mode_->setEnabled(condition_->currentData().toString() == QLatin1String("mode"));
    });
    then_ = new QComboBox(this);
    then_->setObjectName(QStringLiteral("actionsConditionalThen"));
    else_ = new QComboBox(this);
    else_->setObjectName(QStringLiteral("actionsConditionalElse"));
    error_ = makeErrorLabel(this);
    buttons_ = makeButtons(this);

    auto* form = new QFormLayout;
    form->addRow(tr("Condition:"), condition_);
    form->addRow(tr("Mode:"), mode_);
    form->addRow(tr("Then:"), then_);
    form->addRow(tr("Else:"), else_);
    auto* layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(error_);
    layout->addWidget(buttons_);
}

void InsertConditionalDialog::setActions(const QList<ActionDialogSet>& actions)
{
    then_->clear();
    else_->clear();
    then_->addItem(tr("None"), QString());
    else_->addItem(tr("None"), QString());
    for (const ActionDialogSet& action : actions) {
        then_->addItem(action.name, action.id);
        else_->addItem(action.name, action.id);
    }
}

void InsertConditionalDialog::setAllowedActionIds(const QStringList& ids)
{
    allowed_ = ids;
}

QString InsertConditionalDialog::condition() const
{
    const QString kind = condition_->currentData().toString();
    if (kind == QLatin1String("mode")) {
        return QStringLiteral("mode:") + mode_->currentData().toString();
    }
    return kind;
}

QString InsertConditionalDialog::thenActionId() const
{
    return then_->currentData().toString();
}

QString InsertConditionalDialog::elseActionId() const
{
    return else_->currentData().toString();
}

bool InsertConditionalDialog::validate(QString* error) const
{
    const QString thenId = thenActionId();
    const QString elseId = elseActionId();
    if (thenId.isEmpty() && elseId.isEmpty()) {
        if (error) {
            *error = tr("Choose a Then or Else action.");
        }
        return false;
    }
    const bool thenAllowed = thenId.isEmpty() || allowed_.isEmpty() || allowed_.contains(thenId);
    const bool elseAllowed = elseId.isEmpty() || allowed_.isEmpty() || allowed_.contains(elseId);
    if (!thenAllowed || !elseAllowed) {
        if (error) {
            *error = tr("Both actions must belong to this action's set.");
        }
        return false;
    }
    return true;
}

void InsertConditionalDialog::accept()
{
    if (!validate(&lastError_)) {
        showError(error_, buttons_, lastError_);
        return;
    }
    QDialog::accept();
}

} // namespace pictura

#include "moc_actions_dialogs.cpp"
