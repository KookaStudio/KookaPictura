#include "variables_dialog.h"

#include "api/variables_bridge.h"
#include "dialogs.h"
#include "image_view.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QtEndian>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <functional>

namespace pictura::variable_dialogs {

using namespace Qt::StringLiterals;

QJsonObject state(const PictureView& view, bool draft)
{
    return QJsonDocument::fromJson(variables_state(view, draft).toUtf8()).object();
}

QString command(PictureView& view, const QString& action, const QJsonObject& fields)
{
    return variables_command(view, action, QString::fromUtf8(QJsonDocument(fields).toJson(QJsonDocument::Compact)));
}

namespace {

bool change(QWidget* parent, PictureView& view, const QString& action, const QJsonObject& fields = {})
{
    const auto error = command(view, action, fields);
    if (error.isEmpty()) {
        return true;
    }
    QMessageBox::warning(parent, QObject::tr("Variables"), error);
    return false;
}

void addButtons(QDialog& dialog, QVBoxLayout& layout)
{
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout.addWidget(buttons);
}

QString preview(PictureView& view, ImageView* canvas, int set, bool draft)
{
    if (!canvas) {
        return {};
    }
    canvas->clearFilterPreview();
    if (set < 0) {
        return {};
    }
    const auto payload = QJsonDocument(QJsonObject{{u"set"_s, set}, {u"draft"_s, draft}}).toJson(QJsonDocument::Compact);
    const auto bytes = variables_preview(view, QString::fromUtf8(payload));
    if (bytes.size() < 8) {
        return QObject::tr("Preview unavailable.");
    }
    const auto width = qFromLittleEndian<quint32>(bytes.data());
    const auto height = qFromLittleEndian<quint32>(bytes.data() + 4);
    if (width == 0 || height == 0) {
        return QString::fromUtf8(reinterpret_cast<const char*>(bytes.data() + 8), qsizetype(bytes.size() - 8));
    }
    if (bytes.size() - 8 != quint64(width) * height * 4) {
        return QObject::tr("Preview unavailable.");
    }
    const QImage result(bytes.data() + 8, int(width), int(height), int(width) * 4,
                        QImage::Format_RGBA8888_Premultiplied);
    canvas->setFilterPreview(result.copy(), QRect(0, 0, view.document_width(), view.document_height()), true);
    return {};
}

QJsonObject chooseImport(QWidget* parent)
{
    const auto path = QFileDialog::getOpenFileName(parent, QObject::tr("Import Variable Data Sets"), QString(),
        QObject::tr("Delimited text (*.csv *.tsv *.txt);;All files (*)"));
    if (path.isEmpty()) {
        return {};
    }
    QDialog dialog(parent);
    dialog.setWindowTitle(QObject::tr("Import Variable Data Sets"));
    auto* layout = new QVBoxLayout(&dialog);
    auto* firstNames = new QCheckBox(QObject::tr("Use First Column For Data Set Names"), &dialog);
    auto* replace = new QCheckBox(QObject::tr("Replace Existing Data Sets"), &dialog);
    auto* encoding = new QComboBox(&dialog);
    encoding->addItem(QObject::tr("Automatic"), u"auto"_s);
    encoding->addItem(QObject::tr("Unicode (UTF-8)"), u"utf8"_s);
    encoding->addItem(QObject::tr("Unicode (UTF-16 LE)"), u"utf16le"_s);
    encoding->addItem(QObject::tr("Unicode (UTF-16 BE)"), u"utf16be"_s);
    encoding->addItem(QObject::tr("Western (Windows-1252)"), u"windows1252"_s);
    layout->addWidget(firstNames);
    layout->addWidget(replace);
    auto* form = new QFormLayout;
    form->addRow(QObject::tr("Encoding"), encoding);
    layout->addLayout(form);
    addButtons(dialog, *layout);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return {};
    }
    return {{u"path"_s, path}, {u"firstNames"_s, firstNames->isChecked()},
        {u"replace"_s, replace->isChecked()}, {u"encoding"_s, encoding->currentData().toString()}};
}

class Editor final : public QDialog {
public:
    Editor(PictureView& view, ImageView* canvas, QWidget* parent, bool setsPage)
        : QDialog(parent), view_(view), canvas_(canvas)
    {
        setWindowTitle(tr("Variables"));
        setObjectName(u"variablesDialog"_s);
        auto* layout = new QVBoxLayout(this);
        tabs_ = new QTabWidget(this);
        tabs_->setObjectName(u"variablesPages"_s);
        layout->addWidget(tabs_);
        buildDefine();
        buildSets();
        tabs_->setCurrentIndex(setsPage ? 1 : 0);
        connect(tabs_, &QTabWidget::currentChanged, this, [this](int index) {
            if (index == 1) {
                updatePreview();
            } else if (canvas_) {
                canvas_->clearFilterPreview();
            }
        });
        status_ = new QLabel(this);
        status_->setWordWrap(true);
        layout->addWidget(status_);
        addButtons(*this, *layout);
        reload();
    }

    void accept() override
    {
        if (change(this, view_, u"commit"_s)) {
            QDialog::accept();
        }
    }

private:
    QPushButton* button(const QString& label, QBoxLayout& row, const std::function<void()>& action)
    {
        auto* control = new QPushButton(label, this);
        row.addWidget(control);
        connect(control, &QPushButton::clicked, this, action);
        return control;
    }

    void buildDefine()
    {
        auto* page = new QWidget(this);
        auto* layout = new QVBoxLayout(page);
        auto* form = new QFormLayout;
        layers_ = new QComboBox(page);
        kinds_ = new QComboBox(page);
        names_ = new QComboBox(page);
        names_->setObjectName(u"variableBindingName"_s);
        names_->setEditable(true);
        methods_ = new QComboBox(page);
        methods_->addItem(tr("Fit"), u"fit"_s);
        methods_->addItem(tr("Fill"), u"fill"_s);
        methods_->addItem(tr("As Is"), u"asIs"_s);
        methods_->addItem(tr("Conform"), u"conform"_s);
        horizontal_ = new QComboBox(page);
        horizontal_->addItems({tr("Left"), tr("Center"), tr("Right")});
        vertical_ = new QComboBox(page);
        vertical_->addItems({tr("Top"), tr("Middle"), tr("Bottom")});
        clip_ = new QCheckBox(tr("Clip To Bounding Box"), page);
        form->addRow(tr("Layer"), layers_);
        form->addRow(tr("Variable type"), kinds_);
        form->addRow(tr("Name"), names_);
        form->addRow(tr("Method"), methods_);
        form->addRow(tr("Horizontal alignment"), horizontal_);
        form->addRow(tr("Vertical alignment"), vertical_);
        form->addRow(clip_);
        layout->addLayout(form);
        auto* actions = new QHBoxLayout;
        bind_ = button(tr("Set Binding"), *actions, [this] { saveBinding(false); });
        bind_->setObjectName(u"variableSetBinding"_s);
        unbind_ = button(tr("Remove Binding"), *actions, [this] { saveBinding(true); });
        rename_ = button(tr("Rename Variable"), *actions, [this] {
            const auto selected = names_->currentText();
            bool ok = false;
            const auto name = QInputDialog::getText(this, tr("Rename Variable"), tr("Name"), QLineEdit::Normal, selected, &ok);
            if (ok && change(this, view_, u"renameVariable"_s, {{u"from"_s, selected}, {u"name"_s, name}})) {
                reload();
            }
        });
        layout->addLayout(actions);
        layout->addStretch();
        tabs_->addTab(page, tr("Define"));
        connect(layers_, &QComboBox::currentIndexChanged, this, [this] { showKinds(); });
        connect(kinds_, &QComboBox::currentIndexChanged, this, [this] { showDefinition(); });
        connect(methods_, &QComboBox::currentIndexChanged, this, [this] { placementControls(); });
    }

    void buildSets()
    {
        auto* page = new QWidget(this);
        auto* layout = new QVBoxLayout(page);
        auto* form = new QFormLayout;
        sets_ = new QComboBox(page);
        setVariables_ = new QComboBox(page);
        value_ = new QStackedWidget(page);
        value_->addWidget(new QLabel(tr("Select a data set and variable."), page));
        visible_ = new QCheckBox(tr("Visible"), page);
        value_->addWidget(visible_);
        text_ = new QPlainTextEdit(page);
        value_->addWidget(text_);
        auto* image = new QWidget(page);
        auto* imageLayout = new QVBoxLayout(image);
        imagePath_ = new QLineEdit(image);
        imagePath_->setReadOnly(true);
        imageLayout->addWidget(imagePath_);
        auto* imageActions = new QHBoxLayout;
        button(tr("Select File…"), *imageActions, [this] {
            const auto path = QFileDialog::getOpenFileName(this, tr("Replacement Image"));
            if (!path.isEmpty() && editValue(path)) {
                showValue();
            }
        });
        button(tr("Do Not Replace"), *imageActions, [this] {
            if (editValue(QJsonValue::Null)) {
                showValue();
            }
        });
        imageLayout->addLayout(imageActions);
        value_->addWidget(image);
        form->addRow(tr("Data set"), sets_);
        form->addRow(tr("Variable"), setVariables_);
        form->addRow(tr("Value"), value_);
        layout->addLayout(form);
        auto* actions = new QHBoxLayout;
        newSet_ = button(tr("New Data Set"), *actions, [this] {
            QJsonObject values;
            if (sets_->currentIndex() >= 0) {
                values.insert(u"copy"_s, sets_->currentIndex());
            }
            if (change(this, view_, u"newSet"_s, values)) {
                reload();
                sets_->setCurrentIndex(sets_->count() - 1);
            }
        });
        newSet_->setObjectName(u"variableNewDataSet"_s);
        renameSet_ = button(tr("Rename"), *actions, [this] {
            bool ok = false;
            const auto name = QInputDialog::getText(this, tr("Rename Data Set"), tr("Name"), QLineEdit::Normal, sets_->currentText(), &ok);
            if (ok && change(this, view_, u"renameSet"_s, {{u"set"_s, sets_->currentIndex()}, {u"name"_s, name}})) {
                reload();
            }
        });
        deleteSet_ = button(tr("Delete"), *actions, [this] {
            if (change(this, view_, u"deleteSet"_s, {{u"set"_s, sets_->currentIndex()}})) {
                reload();
            }
        });
        import_ = button(tr("Import…"), *actions, [this] {
            const auto fields = chooseImport(this);
            if (!fields.isEmpty() && change(this, view_, u"import"_s, fields)) {
                reload();
            }
        });
        layout->addLayout(actions);
        preview_ = new QCheckBox(tr("Preview"), page);
        preview_->setObjectName(u"dataSetPreview"_s);
        layout->addWidget(preview_);
        tabs_->addTab(page, tr("Data Sets"));
        connect(sets_, &QComboBox::currentIndexChanged, this, [this] {
            if (!loading_) { showValue(); updatePreview(); }
        });
        connect(setVariables_, &QComboBox::currentIndexChanged, this, [this] { if (!loading_) { showValue(); } });
        connect(visible_, &QCheckBox::toggled, this, [this](bool value) { editValue(value); });
        connect(text_, &QPlainTextEdit::textChanged, this, [this] { editValue(text_->toPlainText()); });
        connect(preview_, &QCheckBox::toggled, this, [this] { updatePreview(); });
    }

    void reload()
    {
        loading_ = true;
        const auto path = layers_->currentData();
        const auto variable = setVariables_->currentText();
        const int set = sets_->currentIndex();
        model_ = state(view_, true);
        QStringList issues;
        for (const auto issue : model_.value(u"issues"_s).toArray()) {
            issues.append(issue.toString());
        }
        status_->setText(issues.join(u'\n'));
        status_->setVisible(!issues.isEmpty());

        const auto definitions = model_.value(u"variables"_s).toArray();
        QSet<qint64> bound;
        setVariables_->clear();
        for (const auto& entry : definitions) {
            const auto item = entry.toObject();
            setVariables_->addItem(item.value(u"name"_s).toString(), item.value(u"kind"_s).toString());
            for (const auto& binding : item.value(u"bindings"_s).toArray()) {
                bound.insert(binding.toObject().value(u"id"_s).toInteger());
            }
        }
        setVariables_->setCurrentIndex(std::max(0, setVariables_->findText(variable)));
        layers_->clear();
        for (const auto& entry : model_.value(u"layers"_s).toArray()) {
            const auto item = entry.toObject();
            const auto layerPath = item.value(u"path"_s).toString();
            auto name = QString(layerPath.count(u'/') * 2, u' ') + item.value(u"name"_s).toString();
            if (bound.contains(item.value(u"id"_s).toInteger())) {
                name += u" *"_s;
            }
            layers_->addItem(name, layerPath);
        }
        layers_->setCurrentIndex(std::max(0, layers_->findData(path)));
        sets_->clear();
        for (const auto row : model_.value(u"sets"_s).toArray()) {
            sets_->addItem(row.toObject().value(u"name"_s).toString());
        }
        sets_->setCurrentIndex(sets_->count() > 0 ? std::clamp(set, 0, sets_->count() - 1) : -1);
        const bool editable = issues.isEmpty();
        newSet_->setEnabled(editable && !definitions.isEmpty());
        import_->setEnabled(editable && !definitions.isEmpty());
        loading_ = false;
        showKinds();
        showValue();
        updatePreview();
    }

    void showKinds()
    {
        if (loading_) {
            return;
        }
        const auto previous = kinds_->currentData();
        loading_ = true;
        kinds_->clear();
        const auto layers = model_.value(u"layers"_s).toArray();
        const int index = layers_->currentIndex();
        if (index >= 0 && index < layers.size()) {
            for (const auto kind : layers.at(index).toObject().value(u"allowed"_s).toArray()) {
                const auto value = kind.toString();
                kinds_->addItem(value == u"visibility"_s ? tr("Visibility")
                               : value == u"text"_s ? tr("Text Replacement") : tr("Pixel Replacement"), value);
            }
        }
        kinds_->setCurrentIndex(std::max(0, kinds_->findData(previous)));
        loading_ = false;
        showDefinition();
    }

    void showDefinition()
    {
        if (loading_) {
            return;
        }
        names_->clear();
        const auto kind = kinds_->currentData().toString();
        const auto layers = model_.value(u"layers"_s).toArray();
        const int layer = layers_->currentIndex();
        const auto id = layer >= 0 && layer < layers.size() ? layers.at(layer).toObject().value(u"id"_s).toInteger() : 0;
        QString current;
        QJsonObject options;
        for (const auto& entry : model_.value(u"variables"_s).toArray()) {
            const auto item = entry.toObject();
            if (item.value(u"kind"_s).toString() != kind) {
                continue;
            }
            const auto name = item.value(u"name"_s).toString();
            names_->addItem(name);
            for (const auto& binding : item.value(u"bindings"_s).toArray()) {
                if (binding.toObject().value(u"id"_s).toInteger() == id) {
                    options = binding.toObject();
                    current = name;
                }
            }
        }
        names_->setEditText(current);
        methods_->setCurrentIndex(std::max(0, methods_->findData(options.value(u"method"_s).toString())));
        horizontal_->setCurrentIndex(options.value(u"horizontal"_s).toInt(1));
        vertical_->setCurrentIndex(options.value(u"vertical"_s).toInt(1));
        clip_->setChecked(options.value(u"clip"_s).toBool());
        const bool editable = model_.value(u"issues"_s).toArray().isEmpty();
        bind_->setEnabled(editable && !kind.isEmpty());
        unbind_->setEnabled(editable && !current.isEmpty());
        rename_->setEnabled(editable && names_->count() > 0);
        names_->setEnabled(editable && !kind.isEmpty());
        placementControls();
    }

    void placementControls()
    {
        const bool pixels = kinds_->currentData().toString() == u"pixels"_s;
        const auto method = methods_->currentData().toString();
        methods_->setEnabled(pixels);
        horizontal_->setEnabled(pixels && method != u"conform"_s);
        vertical_->setEnabled(horizontal_->isEnabled());
        clip_->setEnabled(pixels && (method == u"fill"_s || method == u"asIs"_s));
    }

    void saveBinding(bool remove)
    {
        const QJsonObject options{{u"path"_s, layers_->currentData().toString()}, {u"kind"_s, kinds_->currentData().toString()},
            {u"name"_s, names_->currentText()}, {u"method"_s, methods_->currentData().toString()},
            {u"horizontal"_s, horizontal_->currentIndex()}, {u"vertical"_s, vertical_->currentIndex()},
            {u"clip"_s, clip_->isEnabled() && clip_->isChecked()}};
        if (change(this, view_, remove ? u"unbind"_s : u"bind"_s, options)) {
            reload();
        }
    }

    bool editValue(const QJsonValue& value)
    {
        if (loading_ || sets_->currentIndex() < 0 || setVariables_->currentIndex() < 0) {
            return false;
        }
        const auto success = change(this, view_, u"setValue"_s, {{u"set"_s, sets_->currentIndex()},
            {u"name"_s, setVariables_->currentText()}, {u"value"_s, value}});
        if (success && preview_->isChecked()) {
            updatePreview();
        }
        return success;
    }

    void showValue()
    {
        loading_ = true;
        const auto set = sets_->currentIndex();
        const auto kind = setVariables_->currentData().toString();
        const auto rows = state(view_, true).value(u"sets"_s).toArray();
        const auto stored = set >= 0 && set < rows.size()
            ? rows.at(set).toObject().value(u"values"_s).toObject().value(setVariables_->currentText()) : QJsonValue();
        const bool editable = set >= 0 && model_.value(u"issues"_s).toArray().isEmpty();
        renameSet_->setEnabled(editable);
        deleteSet_->setEnabled(editable);
        preview_->setEnabled(editable);
        value_->setEnabled(editable);
        if (set < 0 || kind.isEmpty()) {
            value_->setCurrentIndex(0);
        } else if (kind == u"visibility"_s) {
            value_->setCurrentIndex(1);
            visible_->setChecked(stored.toBool());
        } else if (kind == u"text"_s) {
            value_->setCurrentIndex(2);
            text_->setPlainText(stored.toString());
        } else {
            value_->setCurrentIndex(3);
            imagePath_->setText(stored.toString());
            imagePath_->setPlaceholderText(tr("Do Not Replace"));
        }
        loading_ = false;
    }

    void updatePreview()
    {
        const bool enabled = tabs_->currentIndex() == 1 && preview_->isEnabled() && preview_->isChecked();
        const auto error = preview(view_, canvas_, enabled ? sets_->currentIndex() : -1, true);
        if (model_.value(u"issues"_s).toArray().isEmpty()) {
            status_->setText(error);
            status_->setVisible(!error.isEmpty());
        }
    }

    PictureView& view_;
    QPointer<ImageView> canvas_;
    QJsonObject model_;
    bool loading_ = false;
    QTabWidget* tabs_;
    QLabel* status_;
    QComboBox *layers_, *kinds_, *names_, *methods_, *horizontal_, *vertical_;
    QCheckBox *clip_, *visible_, *preview_;
    QPushButton *bind_, *unbind_, *rename_, *newSet_, *renameSet_, *deleteSet_, *import_;
    QComboBox *sets_, *setVariables_;
    QStackedWidget* value_;
    QPlainTextEdit* text_;
    QLineEdit* imagePath_;
};

} // namespace

void define(PictureView& view, ImageView* canvas, QWidget* parent, bool setsPage)
{
    if (!change(parent, view, u"begin"_s)) {
        return;
    }
    QPointer<ImageView> active(canvas);
    Editor dialog(view, canvas, parent, setsPage);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        command(view, u"cancel"_s);
    }
    if (active) {
        active->clearFilterPreview();
    }
}

void importSets(PictureView& view, QWidget* parent)
{
    const auto fields = chooseImport(parent);
    if (!fields.isEmpty()) {
        change(parent, view, u"importLive"_s, fields);
    }
}

void apply(PictureView& view, ImageView* canvas, QWidget* parent)
{
    QDialog dialog(parent);
    dialog.setWindowTitle(QObject::tr("Apply Data Set"));
    auto* layout = new QVBoxLayout(&dialog);
    auto* sets = new QComboBox(&dialog);
    for (const auto& set : state(view).value(u"sets"_s).toArray()) {
        sets->addItem(set.toObject().value(u"name"_s).toString());
    }
    auto* enabled = new QCheckBox(QObject::tr("Preview"), &dialog);
    auto* status = new QLabel(&dialog);
    status->setWordWrap(true);
    status->hide();
    layout->addWidget(sets);
    layout->addWidget(enabled);
    layout->addWidget(status);
    QPointer<ImageView> active(canvas);
    const auto update = [&] {
        const auto error = preview(view, active, enabled->isChecked() ? sets->currentIndex() : -1, false);
        status->setText(error);
        status->setVisible(!error.isEmpty());
    };
    QObject::connect(enabled, &QCheckBox::toggled, &dialog, update);
    QObject::connect(sets, &QComboBox::currentIndexChanged, &dialog, update);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Apply | QDialogButtonBox::Cancel, &dialog);
    buttons->button(QDialogButtonBox::Apply)->setEnabled(sets->count() != 0);
    QObject::connect(buttons->button(QDialogButtonBox::Apply), &QPushButton::clicked, &dialog, [&] {
        if (change(&dialog, view, u"apply"_s, {{u"set"_s, sets->currentIndex()}})) {
            dialog.accept();
        }
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);
    runDialog(dialog, parent);
    if (active) {
        active->clearFilterPreview();
    }
}

void exportSets(PictureView& view, QWidget* parent, const QString& filePath, const QString& name)
{
    QDialog dialog(parent);
    dialog.setWindowTitle(QObject::tr("Data Sets As Files"));
    dialog.setObjectName(u"dataSetsExportDialog"_s);
    auto* layout = new QVBoxLayout(&dialog);
    auto* form = new QFormLayout;
    auto* folder = new QLineEdit(filePath.isEmpty() ? QDir::homePath() : QFileInfo(filePath).absolutePath(), &dialog);
    auto* pick = new QPushButton(QObject::tr("Select Folder…"), &dialog);
    QObject::connect(pick, &QPushButton::clicked, &dialog, [&] {
        const auto chosen = QFileDialog::getExistingDirectory(&dialog, QObject::tr("Export Folder"), folder->text());
        if (!chosen.isEmpty()) { folder->setText(chosen); }
    });
    auto* destination = new QHBoxLayout;
    destination->addWidget(folder);
    destination->addWidget(pick);
    auto* base = new QLineEdit(filePath.isEmpty() ? name : QFileInfo(filePath).completeBaseName(), &dialog);
    auto* pattern = new QLineEdit(u"{base}_{set}"_s, &dialog);
    pattern->setToolTip(QObject::tr("Use {base}, {set}, and {index} for the base name, data set name, and data set number."));
    auto* list = new QListWidget(&dialog);
    list->setObjectName(u"dataSetsExportSelection"_s);
    for (const auto& set : state(view).value(u"sets"_s).toArray()) {
        auto* item = new QListWidgetItem(set.toObject().value(u"name"_s).toString(), list);
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setCheckState(Qt::Checked);
    }
    form->addRow(QObject::tr("Destination"), destination);
    form->addRow(QObject::tr("Base Name"), base);
    form->addRow(QObject::tr("Naming Pattern"), pattern);
    form->addRow(QObject::tr("Data Sets"), list);
    layout->addLayout(form);
    addButtons(dialog, *layout);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return;
    }
    QJsonArray selected;
    for (int index = 0; index < list->count(); ++index) {
        if (list->item(index)->checkState() == Qt::Checked) { selected.append(index); }
    }
    if (!change(parent, view, u"exportBegin"_s, {{u"folder"_s, folder->text()}, {u"base"_s, base->text()},
        {u"pattern"_s, pattern->text()}, {u"selected"_s, selected}})) {
        return;
    }
    QProgressDialog progress(QObject::tr("Exporting data sets…"), QObject::tr("Cancel"), 0, int(selected.size()), parent);
    progress.setWindowModality(Qt::WindowModal);
    progress.setAutoClose(false);
    progress.setAutoReset(false);
    progress.setMinimumDuration(0);
    progress.setValue(0);
    QJsonObject result;
    while (!progress.wasCanceled() && !result.value(u"done"_s).toBool()) {
        const auto message = command(view, u"exportStep"_s);
        result = QJsonDocument::fromJson(message.toUtf8()).object();
        if (result.isEmpty()) {
            result.insert(u"failure"_s, message);
            break;
        }
        progress.setValue(result.value(u"progress"_s).toInt());
    }
    command(view, u"exportEnd"_s);
    progress.hide();
    QStringList summary;
    if (progress.wasCanceled()) {
        summary.append(QObject::tr("Export cancelled."));
    }
    if (const auto failure = result.value(u"failure"_s).toString(); !failure.isEmpty()) {
        summary.append(failure);
    }
    for (const auto& skipped : result.value(u"skipped"_s).toArray()) {
        summary.append(QObject::tr("Skipped %1").arg(skipped.toString()));
    }
    summary.append(QObject::tr("%n PSD file(s) written.", nullptr, result.value(u"written"_s).toInt()));
    QMessageBox::information(parent, QObject::tr("Data Sets As Files"), summary.join(u'\n'));
}

} // namespace pictura::variable_dialogs
