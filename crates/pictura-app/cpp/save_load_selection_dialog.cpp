#include "save_load_selection_dialog.h"

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr const char* kNewChannel = "new";

} // namespace

SaveLoadSelectionDialog::SaveLoadSelectionDialog(Mode mode,
                                                 const QList<SelectionDocument>& documents,
                                                 QWidget* parent)
    : QDialog(parent)
    , mode_(mode)
    , documents_(documents)
{
    setObjectName(mode_ == Mode::Save ? QStringLiteral("saveSelectionDialog")
                                      : QStringLiteral("loadSelectionDialog"));
    setWindowTitle(mode_ == Mode::Save ? tr("Save Selection") : tr("Load Selection"));

    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    document_ = new QComboBox(this);
    document_->setObjectName(QStringLiteral("selectionSaveLoadDocument"));
    for (int i = 0; i < documents_.size(); ++i) {
        document_->addItem(documents_.at(i).label, i);
    }
    if (mode_ == Mode::Save) {
        document_->addItem(tr("New"), int(documents_.size()));
    }
    for (int i = 0; i < documents_.size(); ++i) {
        if (documents_.at(i).active) {
            document_->setCurrentIndex(i);
            break;
        }
    }
    form->addRow(tr("Document:"), document_);

    channel_ = new QComboBox(this);
    channel_->setObjectName(QStringLiteral("selectionSaveLoadChannel"));
    form->addRow(tr("Channel:"), channel_);

    nameLabel_ = new QLabel(tr("Name:"), this);
    name_ = new QLineEdit(this);
    name_->setObjectName(QStringLiteral("selectionSaveLoadName"));
    form->addRow(nameLabel_, name_);

    operation_ = new QComboBox(this);
    operation_->setObjectName(QStringLiteral("selectionSaveLoadOperation"));
    if (mode_ == Mode::Save) {
        operation_->addItem(tr("Replace Channel"), QStringLiteral("replace"));
        operation_->addItem(tr("Add to Channel"), QStringLiteral("add"));
        operation_->addItem(tr("Subtract From Channel"), QStringLiteral("subtract"));
        operation_->addItem(tr("Intersect With Channel"), QStringLiteral("intersect"));
    } else {
        operation_->addItem(tr("New Selection"), QStringLiteral("new"));
        operation_->addItem(tr("Add To Selection"), QStringLiteral("add"));
        operation_->addItem(tr("Subtract From Selection"), QStringLiteral("subtract"));
        operation_->addItem(tr("Intersect With Selection"), QStringLiteral("intersect"));
    }
    form->addRow(tr("Operation:"), operation_);

    invert_ = new QCheckBox(tr("Invert"), this);
    invert_->setObjectName(QStringLiteral("selectionSaveLoadInvert"));
    form->addRow(QString(), invert_);
    root->addLayout(form);

    buttons_ = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    buttons_->setObjectName(QStringLiteral("selectionSaveLoadButtons"));
    root->addWidget(buttons_);
    connect(buttons_, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons_, &QDialogButtonBox::rejected, this, &QDialog::reject);

    if (mode_ == Mode::Load) {
        nameLabel_->hide();
        name_->hide();
    } else {
        invert_->hide();
    }

    connect(document_, &QComboBox::currentIndexChanged, this,
            [this]() { refreshChannels(); });
    connect(channel_, &QComboBox::currentIndexChanged, this,
            [this]() { refreshEnabled(); });
    connect(operation_, &QComboBox::currentIndexChanged, this,
            [this]() { refreshEnabled(); });
    connect(name_, &QLineEdit::textChanged, this, [this]() { refreshEnabled(); });

    refreshChannels();
}

PictureView* SaveLoadSelectionDialog::selectedView() const
{
    const int index = document_->currentData().toInt();
    if (index < 0 || index >= documents_.size()) {
        return nullptr;
    }
    return documents_.at(index).view;
}

bool SaveLoadSelectionDialog::newDocumentSelected() const
{
    return mode_ == Mode::Save && document_->currentData().toInt() == documents_.size();
}

void SaveLoadSelectionDialog::refreshChannels()
{
    const QSignalBlocker block(channel_);
    channel_->clear();
    if (mode_ == Mode::Save && newDocumentSelected()) {
        channel_->addItem(tr("New Channel"), QString::fromLatin1(kNewChannel));
        refreshEnabled();
        return;
    }
    PictureView* view = selectedView();
    if (mode_ == Mode::Save) {
        channel_->addItem(tr("New Channel"), QString::fromLatin1(kNewChannel));
    }
    if (view && view->has_document()) {
        const QStringList names = channels_alpha_names(*view);
        for (int i = 0; i < names.size(); ++i) {
            channel_->addItem(names.at(i), QStringLiteral("alpha:%1").arg(i));
        }
        for (const QString& target : selection_mask_targets(*view)) {
            const int bar = target.indexOf(QLatin1Char('|'));
            const QString path = bar < 0 ? target : target.left(bar);
            const QString label = bar < 0 ? target : target.mid(bar + 1);
            channel_->addItem(label, QStringLiteral("mask:%1").arg(path));
        }
    }
    if (channel_->count() > 0) {
        channel_->setCurrentIndex(0);
    }
    refreshEnabled();
}

void SaveLoadSelectionDialog::refreshEnabled()
{
    const bool newDocument = newDocumentSelected();
    const bool newChannel = mode_ == Mode::Save
        && (newDocument || channel_->currentData().toString() == QString::fromLatin1(kNewChannel));
    if (mode_ == Mode::Save) {
        name_->setEnabled(newChannel);
        operation_->setEnabled(!newChannel);
    }
    if (mode_ == Mode::Load) {
        invert_->setEnabled(channel_->count() > 0);
    }
    const bool ok = channel_->count() > 0;
    buttons_->button(QDialogButtonBox::Ok)->setEnabled(ok);
}

SelectionSaveRequest SaveLoadSelectionDialog::saveRequest() const
{
    SelectionSaveRequest request;
    request.newDocument = newDocumentSelected();
    request.name = name_->text().trimmed();
    request.mode = operation_->currentData().toString();
    if (request.newDocument) {
        request.target = QString::fromLatin1(kNewChannel);
    } else {
        request.document = selectedView();
        request.target = channel_->currentData().toString();
    }
    return request;
}

SelectionLoadRequest SaveLoadSelectionDialog::loadRequest() const
{
    SelectionLoadRequest request;
    request.document = selectedView();
    request.target = channel_->currentData().toString();
    request.invert = invert_->isChecked();
    request.mode = operation_->currentData().toString();
    return request;
}

int SaveLoadSelectionDialog::documentCountForTest() const { return document_->count(); }

int SaveLoadSelectionDialog::currentDocumentForTest() const
{
    return document_->currentData().toInt();
}

int SaveLoadSelectionDialog::channelCountForTest() const { return channel_->count(); }

QString SaveLoadSelectionDialog::channelTextForTest(int index) const
{
    return index >= 0 && index < channel_->count() ? channel_->itemText(index) : QString();
}

QString SaveLoadSelectionDialog::operationTextForTest(int index) const
{
    return index >= 0 && index < operation_->count() ? operation_->itemText(index) : QString();
}

void SaveLoadSelectionDialog::setDocumentIndexForTest(int index)
{
    document_->setCurrentIndex(index);
}

void SaveLoadSelectionDialog::setChannelIndexForTest(int index)
{
    channel_->setCurrentIndex(index);
}

void SaveLoadSelectionDialog::setOperationIndexForTest(int index)
{
    operation_->setCurrentIndex(index);
}

void SaveLoadSelectionDialog::setNameForTest(const QString& name) { name_->setText(name); }

void SaveLoadSelectionDialog::setInvertForTest(bool invert) { invert_->setChecked(invert); }

bool SaveLoadSelectionDialog::okEnabledForTest() const
{
    return buttons_->button(QDialogButtonBox::Ok)->isEnabled();
}

} // namespace pictura
